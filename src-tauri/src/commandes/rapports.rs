use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

use crate::depot::{compteurs, horloge, rapports, sessions, ventes, Depot};
use crate::domaine::horodatage;
use crate::domaine::ticket::{self, DonneesRapportZ};
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::RegistreJetons;

use super::reglages;

#[derive(Serialize)]
pub struct RapportZ {
    pub donnees: DonneesRapportZ,
    pub texte: String,
    pub colonnes: i64,
    pub session_ouverte: bool,
    pub cloture: bool,
}

/// Lecture seule : ce que la cloture imprimerait, sans rien effacer.
#[tauri::command]
pub fn previsualiser_rapport_z(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<RapportZ> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(|connexion| {
        let prochain = compteurs::lire(connexion, compteurs::REMISE_A_ZERO)? + 1;
        let (donnees, colonnes) = assembler(connexion, prochain)?;
        let session_ouverte = sessions::ouverte(connexion)?.is_some();

        Ok(RapportZ {
            texte: ticket::rendre_rapport_z(&donnees, colonnes),
            donnees,
            colonnes: colonnes as i64,
            session_ouverte,
            cloture: false,
        })
    })
}

/// Rapport Z : imprime toutes les ventes puis efface la memoire des ventes.
/// Ne sont pas touches : les grands totaux, le compteur de remises a zero, le
/// numero de ticket, ni la programmation (rayons, produits, comptes).
#[tauri::command]
pub fn cloturer_rapport_z(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<RapportZ> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(|connexion| {
        let transaction = connexion.unchecked_transaction()?;

        if sessions::ouverte(&transaction)?.is_some() {
            return Err(ErreurApplication::Conflit(
                "une session de caisse est ouverte : la fermer avant la cloture".to_string(),
            ));
        }

        let agregats = ventes::agregats(&transaction)?;

        if agregats.nombre_ventes == 0 {
            return Err(ErreurApplication::Conflit(
                "aucune vente en memoire : rien a cloturer".to_string(),
            ));
        }

        let numero = compteurs::incrementer(&transaction, compteurs::REMISE_A_ZERO, 1)?;
        let (donnees, colonnes) = assembler(&transaction, numero)?;
        let texte = ticket::rendre_rapport_z(&donnees, colonnes);

        rapports::archiver(&transaction, &donnees, &texte)?;
        ventes::purger(&transaction)?;
        sessions::purger_fermees(&transaction)?;

        transaction.commit()?;

        Ok(RapportZ {
            donnees,
            texte,
            colonnes: colonnes as i64,
            session_ouverte: false,
            cloture: true,
        })
    })
}

#[tauri::command]
pub fn lister_rapports_z(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<Vec<rapports::RapportArchive>> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(rapports::lister)
}

fn assembler(connexion: &Connection, numero: i64) -> Resultat<(DonneesRapportZ, usize)> {
    let parametrage = reglages::charger(connexion)?;
    let agregats = ventes::agregats(connexion)?;
    let valeurs = compteurs::lire_tous(connexion)?;
    let maintenant = horloge::maintenant(connexion)?;

    let lire = |cle: &str| -> i64 { valeurs.get(cle).copied().unwrap_or(0) };

    let donnees = DonneesRapportZ {
        numero,
        nom_commerce: parametrage.nom_commerce.clone(),
        devise: parametrage.devise.clone(),
        genere_le: horodatage::lisible(&maintenant),
        debut_le: agregats.debut_le.clone(),
        fin_le: agregats.fin_le.clone(),
        nombre_ventes: agregats.nombre_ventes,
        nombre_articles: agregats.nombre_articles,
        total_centimes: agregats.total_centimes,
        premier_ticket: agregats.premier_ticket,
        dernier_ticket: agregats.dernier_ticket,
        par_caissier: ventes::totaux_par_caissier(connexion)?,
        par_rayon: ventes::totaux_par_rayon(connexion)?,
        ventes: ventes::resumes(connexion)?,
        grand_total_centimes: lire(compteurs::GRAND_TOTAL_CENTIMES),
        grand_total_articles: lire(compteurs::GRAND_TOTAL_ARTICLES),
        grand_total_ventes: lire(compteurs::GRAND_TOTAL_VENTES),
        remises_a_zero: lire(compteurs::REMISE_A_ZERO),
    };

    Ok((
        donnees,
        ticket::borner_colonnes(parametrage.ticket_largeur_colonnes),
    ))
}
