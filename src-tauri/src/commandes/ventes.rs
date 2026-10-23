use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::depot::{catalogue, compteurs, sessions, ventes, Depot};
use crate::domaine::monnaie;
use crate::domaine::panier::{self, LigneChiffree};
use crate::domaine::ticket::{self, DonneesTicket, LigneTicket, TotalNomme};
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::RegistreJetons;

use super::reglages;

#[derive(Debug, Deserialize)]
pub struct DemandeLigne {
    pub produit: i64,
    pub quantite: i64,
}

#[derive(Serialize)]
pub struct TicketEmis {
    pub vente: i64,
    pub numero_ticket: i64,
    pub total_centimes: i64,
    pub recu_centimes: i64,
    pub rendu_centimes: i64,
    pub nombre_articles: i64,
    pub ticket_actif: bool,
    pub impression_auto: bool,
    pub colonnes: i64,
    pub texte: String,
    pub donnees: DonneesTicket,
}

/// Coeur de l encaissement. Les prix ne viennent jamais du front : seuls les
/// identifiants de produits et les quantites traversent le pont, tout le
/// chiffrage est refait ici depuis le catalogue.
#[tauri::command]
pub fn encaisser(
    depot: State<Depot>,
    lignes: Vec<DemandeLigne>,
    recu: String,
) -> Resultat<TicketEmis> {
    if lignes.is_empty() {
        return Err(ErreurApplication::Validation(
            "panier vide : rien a encaisser".to_string(),
        ));
    }

    if lignes.len() > panier::LIGNES_MAXIMALES {
        return Err(ErreurApplication::Validation(format!(
            "panier hors limite : maximum {} lignes",
            panier::LIGNES_MAXIMALES
        )));
    }

    let recu_centimes = monnaie::analyser(&recu)?;

    depot.executer(|connexion| {
        let transaction = connexion.unchecked_transaction()?;
        let session = sessions::exiger_ouverte(&transaction)?;
        let parametrage = reglages::charger(&transaction)?;

        let (ordre, quantites) = fusionner(&lignes)?;

        let mut chiffrees: Vec<LigneChiffree> = Vec::new();
        let mut a_enregistrer: Vec<ventes::LigneAEnregistrer> = Vec::new();

        for identifiant_produit in &ordre {
            let quantite = panier::valider_quantite(quantites[identifiant_produit])?;
            let produit = catalogue::exiger_produit(&transaction, *identifiant_produit)?;

            if !produit.actif {
                return Err(ErreurApplication::Conflit(format!(
                    "{} n est plus en vente",
                    produit.libelle
                )));
            }

            let chiffree = LigneChiffree {
                prix_centimes: produit.prix_centimes,
                quantite,
            };

            a_enregistrer.push(ventes::LigneAEnregistrer {
                produit: produit.identifiant,
                libelle: produit.libelle.clone(),
                code_barres: produit.code_barres.clone(),
                rayon_libelle: produit.rayon_libelle.clone(),
                prix_centimes: produit.prix_centimes,
                quantite,
                total_centimes: panier::total_ligne(chiffree)?,
            });

            chiffrees.push(chiffree);
        }

        let total_centimes = panier::total_panier(&chiffrees)?;
        let nombre_articles = panier::nombre_articles(&chiffrees);
        let rendu_centimes = panier::calculer_rendu(total_centimes, recu_centimes)?;
        let numero_ticket = compteurs::incrementer(&transaction, compteurs::NUMERO_TICKET, 1)?;

        let identifiant = ventes::enregistrer(
            &transaction,
            &ventes::VenteAEnregistrer {
                numero_ticket,
                session: session.identifiant,
                caissier: session.caissier,
                total_centimes,
                recu_centimes,
                rendu_centimes,
                nombre_articles,
            },
            &a_enregistrer,
        )?;

        for ligne in &a_enregistrer {
            catalogue::retirer_du_stock(&transaction, ligne.produit, ligne.quantite)?;
        }

        compteurs::incrementer(&transaction, compteurs::GRAND_TOTAL_CENTIMES, total_centimes)?;
        compteurs::incrementer(&transaction, compteurs::GRAND_TOTAL_ARTICLES, nombre_articles)?;
        compteurs::incrementer(&transaction, compteurs::GRAND_TOTAL_VENTES, 1)?;

        let enregistree = ventes::exiger(&transaction, identifiant)?;

        transaction.commit()?;

        let donnees = DonneesTicket {
            nom_commerce: parametrage.nom_commerce.clone(),
            entete: parametrage.ticket_entete.clone(),
            pied: parametrage.ticket_pied.clone(),
            devise: parametrage.devise.clone(),
            numero_ticket,
            horodatage: enregistree.horodatage.clone(),
            caissier: session.caissier_nom.clone(),
            lignes: a_enregistrer
                .iter()
                .map(|ligne| LigneTicket {
                    libelle: ligne.libelle.clone(),
                    quantite: ligne.quantite,
                    prix_centimes: ligne.prix_centimes,
                    total_centimes: ligne.total_centimes,
                })
                .collect(),
            nombre_articles,
            total_centimes,
            recu_centimes,
            rendu_centimes,
        };

        let colonnes = ticket::borner_colonnes(parametrage.ticket_largeur_colonnes);

        Ok(TicketEmis {
            vente: identifiant,
            numero_ticket,
            total_centimes,
            recu_centimes,
            rendu_centimes,
            nombre_articles,
            ticket_actif: parametrage.ticket_actif,
            impression_auto: parametrage.ticket_impression_auto,
            colonnes: colonnes as i64,
            texte: ticket::rendre_ticket(&donnees, colonnes),
            donnees,
        })
    })
}

#[tauri::command]
pub fn lister_ventes(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    caissier: Option<i64>,
) -> Resultat<Vec<ventes::VenteJournal>> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(|connexion| ventes::journal(connexion, caissier))
}

#[tauri::command]
pub fn recuperer_totaux_caissiers(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<Vec<TotalNomme>> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(ventes::totaux_par_caissier)
}

/// Reimpression d un ticket deja encaisse, reserve au gerant.
#[tauri::command]
pub fn relire_ticket(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    vente: i64,
) -> Resultat<TicketEmis> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(|connexion| {
        let parametrage = reglages::charger(connexion)?;
        let entete = ventes::exiger(connexion, vente)?;
        let lignes = ventes::lignes_de(connexion, vente)?;

        let donnees = DonneesTicket {
            nom_commerce: parametrage.nom_commerce.clone(),
            entete: parametrage.ticket_entete.clone(),
            pied: parametrage.ticket_pied.clone(),
            devise: parametrage.devise.clone(),
            numero_ticket: entete.numero_ticket,
            horodatage: entete.horodatage.clone(),
            caissier: entete.caissier.clone(),
            lignes,
            nombre_articles: entete.nombre_articles,
            total_centimes: entete.total_centimes,
            recu_centimes: entete.recu_centimes,
            rendu_centimes: entete.rendu_centimes,
        };

        let colonnes = ticket::borner_colonnes(parametrage.ticket_largeur_colonnes);

        Ok(TicketEmis {
            vente: entete.identifiant,
            numero_ticket: entete.numero_ticket,
            total_centimes: entete.total_centimes,
            recu_centimes: entete.recu_centimes,
            rendu_centimes: entete.rendu_centimes,
            nombre_articles: entete.nombre_articles,
            ticket_actif: parametrage.ticket_actif,
            impression_auto: false,
            colonnes: colonnes as i64,
            texte: ticket::rendre_ticket(&donnees, colonnes),
            donnees,
        })
    })
}

/// Regroupe les demandes du meme produit en conservant l ordre de scan.
fn fusionner(lignes: &[DemandeLigne]) -> Resultat<(Vec<i64>, HashMap<i64, i64>)> {
    let mut ordre: Vec<i64> = Vec::new();
    let mut quantites: HashMap<i64, i64> = HashMap::new();

    for demande in lignes {
        panier::valider_quantite(demande.quantite)?;

        let cumul = quantites.entry(demande.produit).or_insert_with(|| {
            ordre.push(demande.produit);
            0
        });

        *cumul += demande.quantite;
    }

    Ok((ordre, quantites))
}
