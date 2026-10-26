use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

use crate::depot::{compteurs, horloge, rapports, sessions, ventes, Depot};
use crate::domaine::horodatage;
use crate::domaine::monnaie;
use crate::domaine::ticket::{self, DonneesRapportZ};
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::RegistreJetons;

use super::{reglages, transfert};

/// Separateur du fichier d export : le point virgule est ce qu attend un
/// tableur francophone sous Windows, ou la virgule est deja le separateur
/// decimal.
const SEPARATEUR: char = ';';

/// Marque d octets UTF-8. Sans elle, un tableur ouvert d un double clic lit le
/// fichier dans le jeu de caracteres de Windows et abime les accents du nom du
/// commerce.
const MARQUE_UTF8: char = '\u{feff}';

const EXTENSION_EXPORT: &str = "csv";

#[derive(Serialize)]
pub struct RapportZ {
    pub donnees: DonneesRapportZ,
    pub texte: String,
    pub colonnes: i64,
    pub session_ouverte: bool,
    pub cloture: bool,
}

/// Ce qui est parti sur le disque, et le rapport Z de la periode effacee.
#[derive(Serialize)]
pub struct ExportVentes {
    pub rapport: RapportZ,
    pub nom: String,
    pub chemin: String,
    pub octets: u64,
    /// Nombre de lignes d articles ecrites dans le fichier.
    pub lignes: i64,
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
        let (donnees, colonnes, texte) = preparer_remise_a_zero(&transaction)?;

        remettre_a_zero(&transaction, &donnees, &texte)?;
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

/// Vide la memoire des ventes apres avoir ecrit le detail dans un fichier que
/// le tableur ouvre d un double clic. Le fichier est ecrit AVANT le commit :
/// une ecriture qui echoue laisse la periode intacte.
#[tauri::command]
pub fn exporter_ventes_et_vider(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<ExportVentes> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let dossier = transfert::dossier_par_defaut();

    depot.executer(|connexion| {
        let transaction = connexion.unchecked_transaction()?;
        let (donnees, colonnes, texte) = preparer_remise_a_zero(&transaction)?;
        let detail = ventes::lignes_a_exporter(&transaction)?;
        let tableau = construire_csv(&donnees, &detail);

        remettre_a_zero(&transaction, &donnees, &texte)?;

        std::fs::create_dir_all(&dossier)?;

        // L horodatage brut, pas celui du rapport : le nom du fichier doit
        // commencer par l annee pour que le dossier se trie tout seul.
        let nom = nom_fichier(&horloge::maintenant(&transaction)?);
        let chemin = dossier.join(&nom);

        std::fs::write(&chemin, tableau.as_bytes())?;

        transaction.commit()?;

        Ok(ExportVentes {
            nom,
            chemin: chemin.to_string_lossy().to_string(),
            octets: tableau.len() as u64,
            lignes: detail.len() as i64,
            rapport: RapportZ {
                donnees,
                texte,
                colonnes: colonnes as i64,
                session_ouverte: false,
                cloture: true,
            },
        })
    })
}

/// Trace des ventes annulees de la periode en cours, pour l audit du gerant.
/// La remise a zero l emporte avec les ventes.
#[tauri::command]
pub fn lister_ventes_annulees(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<Vec<ventes::VenteAnnulee>> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(ventes::annulations)
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

/// Verrous communs a toute remise a zero, et numerotation du Z qui l archive.
fn preparer_remise_a_zero(
    transaction: &Connection,
) -> Resultat<(DonneesRapportZ, usize, String)> {
    if sessions::ouverte(transaction)?.is_some() {
        return Err(ErreurApplication::Conflit(
            "une session de caisse est ouverte : la fermer avant la cloture".to_string(),
        ));
    }

    let agregats = ventes::agregats(transaction)?;

    if agregats.nombre_ventes == 0 {
        return Err(ErreurApplication::Conflit(
            "aucune vente en memoire : rien a cloturer".to_string(),
        ));
    }

    let numero = compteurs::incrementer(transaction, compteurs::REMISE_A_ZERO, 1)?;
    let (donnees, colonnes) = assembler(transaction, numero)?;
    let texte = ticket::rendre_rapport_z(&donnees, colonnes);

    Ok((donnees, colonnes, texte))
}

/// Archive le Z puis efface la periode. Les grands totaux, le compteur de
/// remises a zero, le numero de ticket et la programmation restent intacts.
fn remettre_a_zero(
    transaction: &Connection,
    donnees: &DonneesRapportZ,
    texte: &str,
) -> Resultat<()> {
    rapports::archiver(transaction, donnees, texte)?;
    ventes::purger(transaction)?;
    sessions::purger_fermees(transaction)?;

    Ok(())
}

/// `lcos_ventes_20261025-1830.csv` depuis un instant brut `2026-10-25 18:30:05`
/// : le nom porte l annee d abord, donc le tri alphabetique du dossier est
/// deja un tri chronologique.
fn nom_fichier(genere_le: &str) -> String {
    let chiffres: String = genere_le.chars().filter(|c| c.is_ascii_digit()).collect();
    let date = chiffres.get(0..8).unwrap_or("00000000");
    let heure = chiffres.get(8..12).unwrap_or("0000");

    format!("lcos_ventes_{date}-{heure}.{EXTENSION_EXPORT}")
}

/// Un champ qui porte le separateur, un guillemet ou un saut de ligne est
/// protege : un libelle d article est une saisie libre.
fn champ(valeur: &str) -> String {
    if valeur.contains(SEPARATEUR) || valeur.contains('"') || valeur.contains('\n') {
        return format!("\"{}\"", valeur.replace('"', "\"\""));
    }

    valeur.to_string()
}

/// Montant sans separateur de milliers : le tableur doit y lire un nombre, pas
/// un texte.
fn montant(centimes: i64, decimales: i64) -> String {
    monnaie::formater_avec(centimes, decimales).replace(' ', "")
}

fn ligne(cellules: &[String]) -> String {
    let mut texte = cellules.join(&SEPARATEUR.to_string());

    texte.push_str("\r\n");

    texte
}

/// Le fichier porte d abord l identite et les totaux de la periode, puis une
/// ligne par article vendu, puis les totaux par caissier et par rayon : tout
/// ce que la remise a zero fait disparaitre de la base.
fn construire_csv(donnees: &DonneesRapportZ, detail: &[ventes::LigneExport]) -> String {
    let decimales = donnees.decimales;
    let texte = |valeur: &str| champ(valeur);
    let nombre = |valeur: i64| valeur.to_string();
    let somme = |valeur: i64| montant(valeur, decimales);

    let mut sortie = String::new();

    sortie.push(MARQUE_UTF8);
    sortie.push_str(&ligne(&[texte("LCOS - detail des ventes de la periode")]));
    sortie.push_str(&ligne(&[texte("Commerce"), texte(&donnees.nom_commerce)]));
    sortie.push_str(&ligne(&[
        texte("Rapport Z"),
        texte(&format!("no {}", donnees.numero)),
    ]));
    sortie.push_str(&ligne(&[texte("Edite le"), texte(&donnees.genere_le)]));
    sortie.push_str(&ligne(&[texte("Premiere vente"), texte(&donnees.debut_le)]));
    sortie.push_str(&ligne(&[texte("Derniere vente"), texte(&donnees.fin_le)]));
    sortie.push_str(&ligne(&[
        texte("Tickets"),
        texte(&format!(
            "no {} a no {}",
            donnees.premier_ticket, donnees.dernier_ticket
        )),
    ]));
    sortie.push_str(&ligne(&[texte("Ventes"), nombre(donnees.nombre_ventes)]));
    sortie.push_str(&ligne(&[texte("Articles"), nombre(donnees.nombre_articles)]));
    sortie.push_str(&ligne(&[
        texte("Total vente"),
        somme(donnees.total_centimes),
        texte(&donnees.devise),
    ]));
    sortie.push_str(&ligne(&[
        texte("Grand total"),
        somme(donnees.grand_total_centimes),
        texte(&donnees.devise),
    ]));
    sortie.push_str("\r\n");

    sortie.push_str(&ligne(&[texte("Detail des ventes")]));
    sortie.push_str(&ligne(&[
        texte("Ticket"),
        texte("Encaissee le"),
        texte("Caissier"),
        texte("Article"),
        texte("Code barres"),
        texte("Rayon"),
        texte("Quantite"),
        texte("Prix unitaire"),
        texte("Total ligne"),
        texte("Total du ticket"),
        texte("Recu"),
        texte("Rendu"),
    ]));

    for article in detail {
        sortie.push_str(&ligne(&[
            nombre(article.numero_ticket),
            texte(&article.encaissee_le),
            texte(&article.caissier),
            texte(&article.libelle),
            texte(&article.code_barres),
            texte(&article.rayon_libelle),
            nombre(article.quantite),
            somme(article.prix_centimes),
            somme(article.total_centimes),
            somme(article.total_vente_centimes),
            somme(article.recu_centimes),
            somme(article.rendu_centimes),
        ]));
    }

    sortie.push_str("\r\n");
    sortie.push_str(&ligne(&[texte("Totaux par caissier")]));
    sortie.push_str(&ligne(&[texte("Caissier"), texte("Ventes"), texte("Total")]));

    for total in &donnees.par_caissier {
        sortie.push_str(&ligne(&[
            texte(&total.libelle),
            nombre(total.quantite),
            somme(total.total_centimes),
        ]));
    }

    sortie.push_str("\r\n");
    sortie.push_str(&ligne(&[texte("Totaux par rayon")]));
    sortie.push_str(&ligne(&[texte("Rayon"), texte("Quantite"), texte("Total")]));

    for total in &donnees.par_rayon {
        sortie.push_str(&ligne(&[
            texte(&total.libelle),
            nombre(total.quantite),
            somme(total.total_centimes),
        ]));
    }

    sortie
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
        decimales: parametrage.decimales,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domaine::ticket::TotalNomme;

    fn donnees() -> DonneesRapportZ {
        DonneesRapportZ {
            numero: 3,
            nom_commerce: "Chez Ali; et fils".to_string(),
            devise: "EUR".to_string(),
            decimales: 2,
            genere_le: "2026-10-25 18:30:05".to_string(),
            debut_le: "2026-10-25 08:12:00".to_string(),
            fin_le: "2026-10-25 18:04:00".to_string(),
            nombre_ventes: 1,
            nombre_articles: 3,
            total_centimes: 450_000,
            premier_ticket: 7,
            dernier_ticket: 7,
            par_caissier: vec![TotalNomme {
                libelle: "Ali".to_string(),
                quantite: 1,
                total_centimes: 450_000,
            }],
            par_rayon: vec![TotalNomme {
                libelle: "Boissons".to_string(),
                quantite: 3,
                total_centimes: 450_000,
            }],
            ventes: Vec::new(),
            grand_total_centimes: 900_000,
            grand_total_articles: 6,
            grand_total_ventes: 2,
            remises_a_zero: 3,
        }
    }

    fn detail() -> Vec<ventes::LigneExport> {
        vec![ventes::LigneExport {
            numero_ticket: 7,
            encaissee_le: "2026-10-25 18:04:00".to_string(),
            caissier: "Ali".to_string(),
            libelle: "Coca 33cl".to_string(),
            code_barres: "3017620422003".to_string(),
            rayon_libelle: "Boissons".to_string(),
            quantite: 3,
            prix_centimes: 150_000,
            total_centimes: 450_000,
            total_vente_centimes: 450_000,
            recu_centimes: 500_000,
            rendu_centimes: 50_000,
        }]
    }

    #[test]
    fn nomme_le_fichier_par_son_instant() {
        assert_eq!(
            nom_fichier("2026-10-25 18:30:05"),
            "lcos_ventes_20261025-1830.csv"
        );
        assert_eq!(nom_fichier(""), "lcos_ventes_00000000-0000.csv");
    }

    #[test]
    fn protege_les_champs_qui_portent_le_separateur_ou_un_guillemet() {
        assert_eq!(champ("Coca 33cl"), "Coca 33cl");
        assert_eq!(champ("Chez Ali; et fils"), "\"Chez Ali; et fils\"");
        assert_eq!(champ("Jus \"maison"), "\"Jus \"\"maison\"");
    }

    #[test]
    fn ecrit_les_montants_sans_separateur_de_milliers() {
        assert_eq!(montant(450_000, 2), "4500,00");
        assert_eq!(montant(450_000, 0), "4500");
        assert_eq!(montant(0, 2), "0,00");
    }

    #[test]
    fn le_tableau_porte_l_entete_le_detail_et_les_totaux() {
        let sortie = construire_csv(&donnees(), &detail());
        let lignes: Vec<&str> = sortie.split("\r\n").collect();

        // La marque UTF-8 ouvre le fichier, sinon le tableur abime les accents.
        assert!(sortie.starts_with(MARQUE_UTF8));
        assert!(sortie.contains("Total vente;4500,00;EUR"));
        assert!(sortie.contains("Grand total;9000,00;EUR"));
        assert!(sortie.contains("Tickets;no 7 a no 7"));

        // Le nom du commerce porte un point virgule : il doit rester un seul
        // champ, donc etre protege par des guillemets.
        assert!(sortie.contains("Commerce;\"Chez Ali; et fils\""));

        assert!(lignes
            .contains(&"7;2026-10-25 18:04:00;Ali;Coca 33cl;3017620422003;Boissons;3;1500,00;4500,00;4500,00;5000,00;500,00"));
        assert!(lignes.contains(&"Ali;1;4500,00"));
        assert!(lignes.contains(&"Boissons;3;4500,00"));
    }

    #[test]
    fn un_tableau_sans_vente_garde_ses_entetes() {
        let sortie = construire_csv(&donnees(), &[]);

        assert!(sortie.contains("Ticket;Encaissee le;Caissier;Article"));
        assert!(sortie.contains("Totaux par rayon"));
    }
}
