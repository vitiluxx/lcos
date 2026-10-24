mod commandes;
mod depot;
// Rendus publics pour l outil en ligne de commande des licences, qui partage
// le format et la verification avec le logiciel lui-meme : une licence emise
// est relue par le meme code que celui qui la validera sur la caisse.
pub mod domaine;
pub mod erreur;
mod securite;

use tauri::Manager;

const NOM_BASE: &str = "lcos.db";

pub fn executer() {
    tauri::Builder::default()
        .setup(|application| {
            let dossier = application.path().app_data_dir()?;
            let socle = depot::Depot::ouvrir(&dossier.join(NOM_BASE))?;

            socle.executer(depot::migrations::appliquer)?;

            application.manage(socle);
            application.manage(securite::RegistreJetons::nouveau());
            application.manage(securite::Limiteur::nouveau());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commandes::systeme::recuperer_etat_application,
            commandes::horloge::recuperer_horloge,
            commandes::horloge::regler_horloge,
            commandes::horloge::ouvrir_reglages_horloge,
            commandes::authentification::recuperer_etat_protection,
            commandes::authentification::initialiser_gerant,
            commandes::authentification::ouvrir_session_gerant,
            commandes::authentification::fermer_session_gerant,
            commandes::authentification::modifier_secret_gerant,
            commandes::authentification::renommer_gerant,
            commandes::comptes::lister_caissiers_actifs,
            commandes::comptes::lister_caissiers,
            commandes::comptes::creer_caissier,
            commandes::comptes::modifier_caissier,
            commandes::comptes::redefinir_code_caissier,
            commandes::comptes::supprimer_caissier,
            commandes::catalogue::lister_rayons,
            commandes::catalogue::creer_rayon,
            commandes::catalogue::modifier_rayon,
            commandes::catalogue::supprimer_rayon,
            commandes::catalogue::lister_produits,
            commandes::catalogue::creer_produit,
            commandes::catalogue::modifier_produit,
            commandes::catalogue::supprimer_produit,
            commandes::catalogue::trouver_produit_par_code,
            commandes::sessions::recuperer_session_courante,
            commandes::sessions::ouvrir_session_caissier,
            commandes::sessions::fermer_session_caissier,
            commandes::sessions::lister_sessions,
            commandes::ventes::encaisser,
            commandes::ventes::lister_ventes,
            commandes::ventes::recuperer_totaux_caissiers,
            commandes::ventes::relire_ticket,
            commandes::rapports::previsualiser_rapport_z,
            commandes::rapports::cloturer_rapport_z,
            commandes::rapports::lister_rapports_z,
            commandes::reglages::recuperer_reglages,
            commandes::reglages::enregistrer_reglages,
            commandes::reglages::basculer_ticket,
            commandes::reglages::basculer_impression_recu,
            commandes::peripheriques::recuperer_reglages_scanner,
            commandes::peripheriques::enregistrer_reglages_scanner,
            commandes::peripheriques::enregistrer_detection_scanner,
            commandes::peripheriques::analyser_code_barres,
            commandes::impression::recuperer_reglages_impression,
            commandes::impression::detecter_imprimantes,
            commandes::impression::enregistrer_reglages_impression,
            commandes::impression::imprimer_ticket,
            commandes::impression::tester_impression,
            commandes::administration::recuperer_etat_administrateur,
            commandes::administration::activer_lancement_administrateur,
            commandes::administration::desactiver_lancement_administrateur,
            commandes::licence::recuperer_etat_licence,
            commandes::licence::activer_licence,
            commandes::licence::retirer_licence,
            commandes::transfert::recuperer_etat_transfert,
            commandes::transfert::exporter_sauvegarde,
            commandes::transfert::importer_sauvegarde
        ])
        .run(tauri::generate_context!())
        .expect("echec du demarrage de LCOS");
}
