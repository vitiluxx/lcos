/*
 * Parcours complet d une journee de caisse sur une base en memoire : c est le
 * schema SQL et ses contraintes qui sont verifies ici, pas le domaine.
 */
use rusqlite::Connection;

use crate::domaine::secret::Empreinte;

use super::{catalogue, comptes, compteurs, migrations, parametres, rapports, sessions, ventes};

const ROLE_GERANT: &str = "gerant";
const ROLE_CAISSIER: &str = "caissier";

fn base() -> Connection {
    let connexion = Connection::open_in_memory().expect("base en memoire");

    connexion
        .execute_batch("PRAGMA foreign_keys = ON;")
        .expect("integrite referentielle");

    migrations::appliquer(&connexion).expect("migrations");

    connexion
}

fn empreinte_factice() -> Empreinte {
    Empreinte {
        empreinte: "00".repeat(32),
        sel: "11".repeat(16),
        iterations: 1,
    }
}

fn produit_vendable(connexion: &Connection, code: Option<&str>, prix: i64) -> i64 {
    let rayon = catalogue::creer_rayon(connexion, "Boissons", 1).expect("rayon");

    catalogue::creer_produit(connexion, rayon, "Coca 33cl", code, prix, 10, true)
        .expect("produit")
}

#[test]
fn applique_toutes_les_migrations_une_seule_fois() {
    let connexion = base();

    assert_eq!(migrations::nombre_appliquees(&connexion).unwrap(), 5);
    assert_eq!(migrations::appliquer(&connexion).unwrap(), 0);
}

#[test]
fn n_accepte_qu_un_seul_gerant() {
    let connexion = base();
    let empreinte = empreinte_factice();

    comptes::creer(&connexion, "Patron", ROLE_GERANT, &empreinte).expect("premier gerant");

    assert!(comptes::creer(&connexion, "Autre", ROLE_GERANT, &empreinte).is_err());
    assert!(comptes::creer(&connexion, "Ali", ROLE_CAISSIER, &empreinte).is_ok());
    assert!(comptes::creer(&connexion, "Ali", ROLE_CAISSIER, &empreinte).is_err());
}

#[test]
fn refuse_deux_codes_barres_identiques_mais_accepte_plusieurs_sans_code() {
    let connexion = base();
    let rayon = catalogue::creer_rayon(&connexion, "Epicerie", 0).unwrap();

    catalogue::creer_produit(&connexion, rayon, "Riz", Some("3017620422003"), 500, 0, true)
        .expect("premier code");

    assert!(catalogue::creer_produit(
        &connexion,
        rayon,
        "Riz bis",
        Some("3017620422003"),
        500,
        0,
        true
    )
    .is_err());

    catalogue::creer_produit(&connexion, rayon, "Vrac 1", None, 100, 0, false).unwrap();
    catalogue::creer_produit(&connexion, rayon, "Vrac 2", None, 200, 0, false).unwrap();
}

#[test]
fn refuse_de_supprimer_un_rayon_habite() {
    let connexion = base();
    let rayon = catalogue::creer_rayon(&connexion, "Boissons", 0).unwrap();
    let produit =
        catalogue::creer_produit(&connexion, rayon, "Eau", None, 100, 0, true).unwrap();

    assert!(catalogue::supprimer_rayon(&connexion, rayon).is_err());

    catalogue::supprimer_produit(&connexion, produit).unwrap();
    catalogue::supprimer_rayon(&connexion, rayon).unwrap();
}

#[test]
fn n_autorise_qu_une_session_ouverte() {
    let connexion = base();
    let caissier = comptes::creer(&connexion, "Ali", ROLE_CAISSIER, &empreinte_factice()).unwrap();
    let second = comptes::creer(&connexion, "Bea", ROLE_CAISSIER, &empreinte_factice()).unwrap();

    let session = sessions::ouvrir(&connexion, caissier, 5000).unwrap();

    assert!(sessions::ouvrir(&connexion, second, 0).is_err());

    sessions::fermer(&connexion, session).unwrap();

    assert!(sessions::ouvrir(&connexion, second, 0).is_ok());
    assert!(sessions::fermer(&connexion, session).is_err());
}

#[test]
fn enregistre_une_vente_et_tient_les_totaux() {
    let connexion = base();
    let caissier = comptes::creer(&connexion, "Ali", ROLE_CAISSIER, &empreinte_factice()).unwrap();
    let produit = produit_vendable(&connexion, Some("3017620422003"), 150);
    let session = sessions::ouvrir(&connexion, caissier, 0).unwrap();

    let numero = compteurs::incrementer(&connexion, compteurs::NUMERO_TICKET, 1).unwrap();

    assert_eq!(numero, 1);

    let lignes = vec![ventes::LigneAEnregistrer {
        produit,
        libelle: "Coca 33cl".to_string(),
        code_barres: Some("3017620422003".to_string()),
        rayon_libelle: "Boissons".to_string(),
        prix_centimes: 150,
        quantite: 2,
        total_centimes: 300,
    }];

    let identifiant = ventes::enregistrer(
        &connexion,
        &ventes::VenteAEnregistrer {
            numero_ticket: numero,
            session,
            caissier,
            total_centimes: 300,
            recu_centimes: 500,
            rendu_centimes: 200,
            nombre_articles: 2,
        },
        &lignes,
    )
    .unwrap();

    catalogue::retirer_du_stock(&connexion, produit, 2).unwrap();
    compteurs::incrementer(&connexion, compteurs::GRAND_TOTAL_CENTIMES, 300).unwrap();
    compteurs::incrementer(&connexion, compteurs::GRAND_TOTAL_VENTES, 1).unwrap();

    let agregats = ventes::agregats(&connexion).unwrap();

    assert_eq!(agregats.nombre_ventes, 1);
    assert_eq!(agregats.total_centimes, 300);
    assert_eq!(agregats.nombre_articles, 2);
    assert_eq!(agregats.premier_ticket, 1);

    assert_eq!(catalogue::exiger_produit(&connexion, produit).unwrap().stock, 8);

    let journal = ventes::journal(&connexion, Some(caissier)).unwrap();

    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0].caissier, "Ali");

    let detail = ventes::lignes_de(&connexion, identifiant).unwrap();

    assert_eq!(detail.len(), 1);
    assert_eq!(detail[0].total_centimes, 300);

    let par_rayon = ventes::totaux_par_rayon(&connexion).unwrap();

    assert_eq!(par_rayon.len(), 1);
    assert_eq!(par_rayon[0].libelle, "Boissons");
    assert_eq!(par_rayon[0].quantite, 2);

    let session_lue = sessions::exiger_ouverte(&connexion).unwrap();

    assert_eq!(session_lue.nombre_ventes, 1);
    assert_eq!(session_lue.total_centimes, 300);
}

#[test]
fn le_stock_libre_n_est_pas_decremente() {
    let connexion = base();
    let rayon = catalogue::creer_rayon(&connexion, "Vrac", 0).unwrap();
    let produit =
        catalogue::creer_produit(&connexion, rayon, "Sucre", None, 100, 7, false).unwrap();

    catalogue::retirer_du_stock(&connexion, produit, 3).unwrap();

    assert_eq!(catalogue::exiger_produit(&connexion, produit).unwrap().stock, 7);
}

#[test]
fn la_remise_a_zero_efface_les_ventes_et_garde_la_programmation() {
    let connexion = base();
    let gerant = comptes::creer(&connexion, "Patron", ROLE_GERANT, &empreinte_factice()).unwrap();
    let caissier = comptes::creer(&connexion, "Ali", ROLE_CAISSIER, &empreinte_factice()).unwrap();
    let produit = produit_vendable(&connexion, Some("3017620422003"), 150);
    let session = sessions::ouvrir(&connexion, caissier, 0).unwrap();

    for rang in 1..=3 {
        let numero = compteurs::incrementer(&connexion, compteurs::NUMERO_TICKET, 1).unwrap();

        ventes::enregistrer(
            &connexion,
            &ventes::VenteAEnregistrer {
                numero_ticket: numero,
                session,
                caissier,
                total_centimes: 150 * rang,
                recu_centimes: 150 * rang,
                rendu_centimes: 0,
                nombre_articles: rang,
            },
            &[ventes::LigneAEnregistrer {
                produit,
                libelle: "Coca 33cl".to_string(),
                code_barres: None,
                rayon_libelle: "Boissons".to_string(),
                prix_centimes: 150,
                quantite: rang,
                total_centimes: 150 * rang,
            }],
        )
        .unwrap();

        compteurs::incrementer(&connexion, compteurs::GRAND_TOTAL_CENTIMES, 150 * rang).unwrap();
        compteurs::incrementer(&connexion, compteurs::GRAND_TOTAL_ARTICLES, rang).unwrap();
        compteurs::incrementer(&connexion, compteurs::GRAND_TOTAL_VENTES, 1).unwrap();
    }

    sessions::fermer(&connexion, session).unwrap();

    let agregats = ventes::agregats(&connexion).unwrap();

    assert_eq!(agregats.nombre_ventes, 3);
    assert_eq!(agregats.total_centimes, 900);

    let numero_raz = compteurs::incrementer(&connexion, compteurs::REMISE_A_ZERO, 1).unwrap();

    assert_eq!(numero_raz, 1);

    let archive = crate::domaine::ticket::DonneesRapportZ {
        numero: numero_raz,
        nom_commerce: "Mon commerce".to_string(),
        devise: "EUR".to_string(),
        genere_le: "24/10/2026 01:12".to_string(),
        debut_le: agregats.debut_le.clone(),
        fin_le: agregats.fin_le.clone(),
        nombre_ventes: agregats.nombre_ventes,
        nombre_articles: agregats.nombre_articles,
        total_centimes: agregats.total_centimes,
        premier_ticket: agregats.premier_ticket,
        dernier_ticket: agregats.dernier_ticket,
        par_caissier: ventes::totaux_par_caissier(&connexion).unwrap(),
        par_rayon: ventes::totaux_par_rayon(&connexion).unwrap(),
        ventes: ventes::resumes(&connexion).unwrap(),
        grand_total_centimes: compteurs::lire(&connexion, compteurs::GRAND_TOTAL_CENTIMES).unwrap(),
        grand_total_articles: compteurs::lire(&connexion, compteurs::GRAND_TOTAL_ARTICLES).unwrap(),
        grand_total_ventes: compteurs::lire(&connexion, compteurs::GRAND_TOTAL_VENTES).unwrap(),
        remises_a_zero: numero_raz,
    };

    rapports::archiver(&connexion, &archive, "texte du rapport").unwrap();

    assert_eq!(ventes::purger(&connexion).unwrap(), 3);
    assert_eq!(sessions::purger_fermees(&connexion).unwrap(), 1);

    // Les ventes ont disparu...
    let apres = ventes::agregats(&connexion).unwrap();

    assert_eq!(apres.nombre_ventes, 0);
    assert_eq!(apres.total_centimes, 0);

    let lignes_restantes: i64 = connexion
        .query_row("SELECT COUNT(*) FROM ligne_vente", [], |ligne| ligne.get(0))
        .unwrap();

    assert_eq!(lignes_restantes, 0);

    // ... mais rien d autre n a bouge.
    assert_eq!(
        compteurs::lire(&connexion, compteurs::NUMERO_TICKET).unwrap(),
        3
    );
    assert_eq!(
        compteurs::lire(&connexion, compteurs::GRAND_TOTAL_CENTIMES).unwrap(),
        900
    );
    assert_eq!(
        compteurs::lire(&connexion, compteurs::REMISE_A_ZERO).unwrap(),
        1
    );
    assert_eq!(rapports::lister(&connexion).unwrap().len(), 1);
    assert_eq!(comptes::lister(&connexion, ROLE_CAISSIER).unwrap().len(), 1);
    assert!(comptes::lire(&connexion, gerant).unwrap().is_some());
    assert_eq!(catalogue::lister_rayons(&connexion, true).unwrap().len(), 1);
    assert_eq!(
        catalogue::lister_produits(&connexion, &catalogue::Filtre::default())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        parametres::lire(&connexion, "nom_commerce").unwrap().unwrap(),
        "Mon commerce"
    );
}

#[test]
fn protege_un_caissier_qui_a_des_ventes() {
    let connexion = base();
    let caissier = comptes::creer(&connexion, "Ali", ROLE_CAISSIER, &empreinte_factice()).unwrap();
    let produit = produit_vendable(&connexion, None, 150);
    let session = sessions::ouvrir(&connexion, caissier, 0).unwrap();

    ventes::enregistrer(
        &connexion,
        &ventes::VenteAEnregistrer {
            numero_ticket: 1,
            session,
            caissier,
            total_centimes: 150,
            recu_centimes: 150,
            rendu_centimes: 0,
            nombre_articles: 1,
        },
        &[ventes::LigneAEnregistrer {
            produit,
            libelle: "Coca 33cl".to_string(),
            code_barres: None,
            rayon_libelle: "Boissons".to_string(),
            prix_centimes: 150,
            quantite: 1,
            total_centimes: 150,
        }],
    )
    .unwrap();

    assert!(comptes::supprimer(&connexion, caissier).is_err());

    comptes::definir_actif(&connexion, caissier, false).unwrap();

    assert!(!comptes::exiger(&connexion, caissier).unwrap().actif);
    assert_eq!(comptes::exiger(&connexion, caissier).unwrap().nombre_ventes, 1);
}

#[test]
fn une_vente_supprimee_conserve_le_libelle_du_produit_efface() {
    let connexion = base();
    let caissier = comptes::creer(&connexion, "Ali", ROLE_CAISSIER, &empreinte_factice()).unwrap();
    let produit = produit_vendable(&connexion, None, 150);
    let session = sessions::ouvrir(&connexion, caissier, 0).unwrap();

    let vente = ventes::enregistrer(
        &connexion,
        &ventes::VenteAEnregistrer {
            numero_ticket: 1,
            session,
            caissier,
            total_centimes: 150,
            recu_centimes: 150,
            rendu_centimes: 0,
            nombre_articles: 1,
        },
        &[ventes::LigneAEnregistrer {
            produit,
            libelle: "Coca 33cl".to_string(),
            code_barres: None,
            rayon_libelle: "Boissons".to_string(),
            prix_centimes: 150,
            quantite: 1,
            total_centimes: 150,
        }],
    )
    .unwrap();

    catalogue::supprimer_produit(&connexion, produit).unwrap();

    let detail = ventes::lignes_de(&connexion, vente).unwrap();

    assert_eq!(detail[0].libelle, "Coca 33cl");
    assert_eq!(detail[0].prix_centimes, 150);
}
