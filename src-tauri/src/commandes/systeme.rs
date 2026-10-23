use serde::Serialize;
use tauri::State;

use crate::depot::{migrations, Depot};
use crate::erreur::Resultat;

#[derive(Serialize)]
pub struct EtatApplication {
    pub version: String,
    pub nom_produit: String,
    pub chemin_base: String,
    pub migrations_appliquees: i64,
    pub parametres_charges: i64,
    pub base_operationnelle: bool,
}

#[tauri::command]
pub fn recuperer_etat_application(depot: State<Depot>) -> Resultat<EtatApplication> {
    let (migrations_appliquees, parametres_charges) = depot.executer(|connexion| {
        let total_migrations = migrations::nombre_appliquees(connexion)?;
        let total_parametres: i64 =
            connexion.query_row("SELECT COUNT(*) FROM parametre", [], |ligne| ligne.get(0))?;

        Ok((total_migrations, total_parametres))
    })?;

    Ok(EtatApplication {
        version: env!("CARGO_PKG_VERSION").to_string(),
        nom_produit: "LCOS".to_string(),
        chemin_base: depot.chemin().display().to_string(),
        migrations_appliquees,
        parametres_charges,
        base_operationnelle: migrations_appliquees > 0,
    })
}
