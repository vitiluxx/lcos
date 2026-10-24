use serde::Serialize;
use tauri::State;

use crate::depot::{comptes, compteurs, horloge, migrations, sessions, Depot};
use crate::domaine::horodatage;
use crate::erreur::Resultat;
use crate::securite::jetons::{ROLE_CAISSIER, ROLE_GERANT};

use super::{licence, reglages};

#[derive(Serialize)]
pub struct EtatApplication {
    pub version: String,
    pub nom_produit: String,
    pub chemin_base: String,
    pub migrations_appliquees: i64,
    pub parametres_charges: i64,
    pub base_operationnelle: bool,
    pub nom_commerce: String,
    pub devise: String,
    pub ticket_actif: bool,
    pub gerant_configure: bool,
    pub session_ouverte: bool,
    pub caissier_en_poste: String,
    pub nombre_caissiers: i64,
    pub nombre_rayons: i64,
    pub nombre_produits: i64,
    pub ventes_en_memoire: i64,
    pub numero_ticket: i64,
    pub remises_a_zero: i64,
    pub horodatage: String,
    pub prete: bool,
    pub licence: licence::EtatLicence,
}

#[tauri::command]
pub fn recuperer_etat_application(depot: State<Depot>) -> Resultat<EtatApplication> {
    let etat = depot.executer(|connexion| {
        let migrations_appliquees = migrations::nombre_appliquees(connexion)?;
        let parametrage = reglages::charger(connexion)?;
        let session = sessions::ouverte(connexion)?;
        let valeurs = compteurs::lire_tous(connexion)?;

        let compter = |requete: &str| -> Resultat<i64> {
            let total = connexion.query_row(requete, [], |ligne| ligne.get(0))?;

            Ok(total)
        };

        let nombre_caissiers = comptes::lister(connexion, ROLE_CAISSIER)?.len() as i64;
        let gerant_configure = comptes::lire_par_role(connexion, ROLE_GERANT)?.is_some();
        let nombre_produits = compter("SELECT COUNT(*) FROM produit")?;
        let maintenant = horloge::maintenant(connexion)?;

        Ok(EtatApplication {
            version: env!("CARGO_PKG_VERSION").to_string(),
            nom_produit: "LCOS".to_string(),
            chemin_base: String::new(),
            migrations_appliquees,
            parametres_charges: compter("SELECT COUNT(*) FROM parametre")?,
            base_operationnelle: migrations_appliquees > 0,
            nom_commerce: parametrage.nom_commerce,
            devise: parametrage.devise,
            ticket_actif: parametrage.ticket_actif,
            gerant_configure,
            session_ouverte: session.is_some(),
            caissier_en_poste: session
                .map(|ouverte| ouverte.caissier_nom)
                .unwrap_or_default(),
            nombre_caissiers,
            nombre_rayons: compter("SELECT COUNT(*) FROM rayon")?,
            nombre_produits,
            ventes_en_memoire: compter("SELECT COUNT(*) FROM vente")?,
            numero_ticket: valeurs
                .get(compteurs::NUMERO_TICKET)
                .copied()
                .unwrap_or_default(),
            remises_a_zero: valeurs
                .get(compteurs::REMISE_A_ZERO)
                .copied()
                .unwrap_or_default(),
            horodatage: horodatage::lisible(&maintenant),
            prete: gerant_configure && nombre_caissiers > 0 && nombre_produits > 0,
            licence: licence::evaluer(connexion)?,
        })
    })?;

    Ok(EtatApplication {
        chemin_base: depot.chemin().display().to_string(),
        ..etat
    })
}
