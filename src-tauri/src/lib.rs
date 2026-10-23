mod commandes;
mod depot;
mod domaine;
mod erreur;

use tauri::Manager;

const NOM_BASE: &str = "lcos.db";

pub fn executer() {
    tauri::Builder::default()
        .setup(|application| {
            let dossier = application.path().app_data_dir()?;
            let socle = depot::Depot::ouvrir(&dossier.join(NOM_BASE))?;

            socle.executer(|connexion| depot::migrations::appliquer(connexion))?;
            application.manage(socle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commandes::systeme::recuperer_etat_application,
            commandes::peripheriques::recuperer_reglages_scanner,
            commandes::peripheriques::enregistrer_reglages_scanner,
            commandes::peripheriques::enregistrer_detection_scanner,
            commandes::peripheriques::analyser_code_barres
        ])
        .run(tauri::generate_context!())
        .expect("echec du demarrage de LCOS");
}
