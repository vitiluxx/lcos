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

use super::{licence, reglages};

const RAYON_ARTICLE_LIBRE: &str = "Divers";
const LONGUEUR_LIBELLE_LIBRE: usize = 32;

/// Une ligne du panier. Sans `produit`, c est un article saisi a la main en
/// mode manuel : seul son prix le definit, il n a pas de fiche catalogue.
#[derive(Debug, Deserialize)]
pub struct DemandeLigne {
    pub produit: Option<i64>,
    pub quantite: i64,
    pub prix: Option<String>,
    pub libelle: Option<String>,
}

struct ArticleLibre {
    libelle: String,
    prix_centimes: i64,
    quantite: i64,
}

/// Panier demande par le front, separe en articles du catalogue (fusionnes
/// par produit) et articles saisis a la main (jamais fusionnes).
struct PanierDemande {
    ordre: Vec<i64>,
    quantites: HashMap<i64, i64>,
    libres: Vec<ArticleLibre>,
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
    pub impression_recu: bool,
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
        // Droit d usage verifie ici, pas seulement a l ecran : c est la vente
        // qui est fermee quand l essai est epuise sans licence.
        licence::exiger_active(connexion)?;

        let transaction = connexion.unchecked_transaction()?;
        let session = sessions::exiger_ouverte(&transaction)?;
        let parametrage = reglages::charger(&transaction)?;

        let demande = fusionner(&lignes)?;
        let PanierDemande {
            ordre,
            quantites,
            libres,
        } = demande;

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
                produit: Some(produit.identifiant),
                libelle: produit.libelle.clone(),
                code_barres: produit.code_barres.clone(),
                rayon_libelle: produit.rayon_libelle.clone(),
                prix_centimes: produit.prix_centimes,
                quantite,
                total_centimes: panier::total_ligne(chiffree)?,
            });

            chiffrees.push(chiffree);
        }

        for article in &libres {
            let chiffree = LigneChiffree {
                prix_centimes: article.prix_centimes,
                quantite: article.quantite,
            };

            a_enregistrer.push(ventes::LigneAEnregistrer {
                produit: None,
                libelle: article.libelle.clone(),
                code_barres: None,
                rayon_libelle: RAYON_ARTICLE_LIBRE.to_string(),
                prix_centimes: article.prix_centimes,
                quantite: article.quantite,
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
            if let Some(identifiant_produit) = ligne.produit {
                catalogue::retirer_du_stock(&transaction, identifiant_produit, ligne.quantite)?;
            }
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
            decimales: parametrage.decimales,
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
            impression_recu: parametrage.impression_recu,
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
            decimales: parametrage.decimales,
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
            impression_recu: false,
            colonnes: colonnes as i64,
            texte: ticket::rendre_ticket(&donnees, colonnes),
            donnees,
        })
    })
}

/// Regroupe les demandes du meme produit en conservant l ordre de scan. Les
/// articles libres ne sont jamais fusionnes : deux prix identiques restent
/// deux lignes distinctes sur le ticket.
fn fusionner(lignes: &[DemandeLigne]) -> Resultat<PanierDemande> {
    let mut ordre: Vec<i64> = Vec::new();
    let mut quantites: HashMap<i64, i64> = HashMap::new();
    let mut libres: Vec<ArticleLibre> = Vec::new();

    for demande in lignes {
        panier::valider_quantite(demande.quantite)?;

        let Some(identifiant) = demande.produit else {
            libres.push(article_libre(demande, libres.len() + 1)?);
            continue;
        };

        let cumul = quantites.entry(identifiant).or_insert_with(|| {
            ordre.push(identifiant);
            0
        });

        *cumul += demande.quantite;
    }

    Ok(PanierDemande {
        ordre,
        quantites,
        libres,
    })
}

fn article_libre(demande: &DemandeLigne, rang: usize) -> Resultat<ArticleLibre> {
    let saisie = demande.prix.as_deref().unwrap_or("");
    let prix_centimes = monnaie::analyser(saisie)?;

    if prix_centimes <= 0 {
        return Err(ErreurApplication::Validation(
            "article manuel : prix superieur a zero attendu".to_string(),
        ));
    }

    let libelle = demande
        .libelle
        .as_deref()
        .map(str::trim)
        .filter(|texte| !texte.is_empty())
        .map(|texte| {
            texte
                .chars()
                .filter(|caractere| !caractere.is_control())
                .take(LONGUEUR_LIBELLE_LIBRE)
                .collect::<String>()
        })
        .unwrap_or_else(|| format!("Article {rang}"));

    Ok(ArticleLibre {
        libelle,
        prix_centimes,
        quantite: demande.quantite,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn du_catalogue(produit: i64, quantite: i64) -> DemandeLigne {
        DemandeLigne {
            produit: Some(produit),
            quantite,
            prix: None,
            libelle: None,
        }
    }

    fn libre(prix: &str, libelle: Option<&str>) -> DemandeLigne {
        DemandeLigne {
            produit: None,
            quantite: 1,
            prix: Some(prix.to_string()),
            libelle: libelle.map(str::to_string),
        }
    }

    #[test]
    fn fusionne_le_catalogue_et_garde_les_articles_libres_separes() {
        let demande = fusionner(&[
            du_catalogue(7, 1),
            libre("1,50", None),
            du_catalogue(7, 2),
            libre("1,50", None),
        ])
        .unwrap();

        assert_eq!(demande.ordre, vec![7]);
        assert_eq!(demande.quantites[&7], 3);
        assert_eq!(demande.libres.len(), 2);
        assert_eq!(demande.libres[0].libelle, "Article 1");
        assert_eq!(demande.libres[1].libelle, "Article 2");
        assert_eq!(demande.libres[0].prix_centimes, 150);
    }

    #[test]
    fn refuse_un_article_libre_sans_prix_utilisable() {
        assert!(fusionner(&[libre("0", None)]).is_err());
        assert!(fusionner(&[libre("", None)]).is_err());
        assert!(fusionner(&[libre("abc", None)]).is_err());
    }

    #[test]
    fn nettoie_le_libelle_d_un_article_libre() {
        let demande = fusionner(&[libre("2,00", Some("  Pain\nchaud  "))]).unwrap();

        assert_eq!(demande.libres[0].libelle, "Painchaud");
    }

    #[test]
    fn borne_la_longueur_du_libelle_libre() {
        let long = "x".repeat(LONGUEUR_LIBELLE_LIBRE + 20);
        let demande = fusionner(&[libre("2,00", Some(&long))]).unwrap();

        assert_eq!(demande.libres[0].libelle.chars().count(), LONGUEUR_LIBELLE_LIBRE);
    }
}
