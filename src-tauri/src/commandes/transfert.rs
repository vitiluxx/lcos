/*
 * Export et import du contenu de la caisse, en un seul fichier JSON lisible a
 * l oeil et relu par le logiciel.
 *
 * Deux portees :
 *   - parametrage seul : reglages, comptes, rayons, produits. C est le fichier
 *     qu on prepare sur un poste et qu on porte sur la caisse ;
 *   - avec les donnees : le parametrage plus les compteurs, les sessions, les
 *     ventes et les rapports Z. C est une sauvegarde de restauration.
 *
 * Aucune boite de dialogue Windows : le gerant designe un dossier, LCOS y
 * ecrit et y relit. Une caisse tactile n a pas de clavier confortable pour un
 * selecteur de fichiers, et la cle USB se monte toujours sur la meme lettre.
 */
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::depot::{horloge, sessions, transfert, Depot};
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::RegistreJetons;

const LOGICIEL: &str = "LCOS";
const FORMAT: i64 = 1;

const CONTENU_PARAMETRAGE: &str = "parametrage";
const CONTENU_COMPLET: &str = "complet";

const DOSSIER_SAUVEGARDES: &str = "LCOS";
const EXTENSION: &str = "json";
const TAILLE_MAXIMALE: u64 = 16 * 1024 * 1024;
const FICHIERS_LISTES: usize = 60;

/// Contenu exact du fichier. L entete est en premier pour qu un simple coup
/// d oeil dans un editeur de texte dise de quoi il s agit.
#[derive(Debug, Serialize, Deserialize)]
pub struct Sauvegarde {
    pub logiciel: String,
    pub format: i64,
    pub version_logiciel: String,
    pub genere_le: String,
    pub contenu: String,
    pub parametrage: transfert::Parametrage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub donnees: Option<transfert::Donnees>,
}

/// Entete seule : sert a decrire un fichier dans la liste sans charger tout
/// son contenu en memoire.
#[derive(Debug, Default, Deserialize)]
struct Entete {
    #[serde(default)]
    logiciel: String,
    #[serde(default)]
    format: i64,
    #[serde(default)]
    version_logiciel: String,
    #[serde(default)]
    genere_le: String,
    #[serde(default)]
    contenu: String,
}

#[derive(Debug, Serialize)]
pub struct FichierSauvegarde {
    pub nom: String,
    pub chemin: String,
    pub octets: u64,
    pub contenu: String,
    pub genere_le: String,
    pub version_logiciel: String,
    /// Faux si le fichier n est pas une sauvegarde LCOS relisible : il reste
    /// affiche, mais le bouton d import le refusera.
    pub utilisable: bool,
}

#[derive(Debug, Serialize)]
pub struct EtatTransfert {
    pub dossier: String,
    pub dossier_defaut: String,
    pub resume: transfert::Resume,
    pub ventes_en_memoire: i64,
    pub session_ouverte: bool,
    pub fichiers: Vec<FichierSauvegarde>,
}

#[derive(Debug, Serialize)]
pub struct ResultatTransfert {
    pub chemin: String,
    pub nom: String,
    pub octets: u64,
    pub contenu: String,
    pub resume: transfert::Resume,
}

#[tauri::command]
pub fn recuperer_etat_transfert(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    dossier: Option<String>,
) -> Resultat<EtatTransfert> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let cible = resoudre_dossier(dossier)?;

    depot.executer(|connexion| {
        let resume = transfert::resumer(connexion)?;

        Ok(EtatTransfert {
            dossier: cible.to_string_lossy().to_string(),
            dossier_defaut: dossier_par_defaut().to_string_lossy().to_string(),
            ventes_en_memoire: resume.ventes,
            resume,
            session_ouverte: sessions::ouverte(connexion)?.is_some(),
            fichiers: lister(&cible),
        })
    })
}

#[tauri::command]
pub fn exporter_sauvegarde(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    dossier: Option<String>,
    avec_donnees: bool,
) -> Resultat<ResultatTransfert> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let cible = resoudre_dossier(dossier)?;

    std::fs::create_dir_all(&cible)?;

    let (sauvegarde, resume) = depot.executer(|connexion| {
        let parametrage = transfert::lire_parametrage(connexion)?;
        let donnees = if avec_donnees {
            Some(transfert::lire_donnees(connexion)?)
        } else {
            None
        };
        let resume = transfert::resumer_contenu(&parametrage, donnees.as_ref());

        Ok((
            Sauvegarde {
                logiciel: LOGICIEL.to_string(),
                format: FORMAT,
                version_logiciel: env!("CARGO_PKG_VERSION").to_string(),
                genere_le: horloge::maintenant(connexion)?,
                contenu: portee(avec_donnees).to_string(),
                parametrage,
                donnees,
            },
            resume,
        ))
    })?;

    let texte = serde_json::to_string_pretty(&sauvegarde).map_err(|erreur| {
        ErreurApplication::Conflit(format!("fichier non ecrit : {erreur}"))
    })?;

    let nom = nom_fichier(&sauvegarde.contenu, &sauvegarde.genere_le);
    let chemin = cible.join(&nom);

    std::fs::write(&chemin, texte.as_bytes())?;

    Ok(ResultatTransfert {
        chemin: chemin.to_string_lossy().to_string(),
        nom,
        octets: texte.len() as u64,
        contenu: sauvegarde.contenu,
        resume,
    })
}

/// Import : tout ce que le fichier contient remplace ce qui est en base, en
/// une seule transaction. Rien de partiel, rien de fusionne : une
/// programmation a moitie remplacee serait pire qu une programmation absente.
#[tauri::command]
pub fn importer_sauvegarde(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    chemin: String,
    avec_donnees: bool,
) -> Resultat<ResultatTransfert> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let fichier = valider_chemin(&chemin)?;
    let octets = std::fs::metadata(&fichier)?.len();
    let texte = std::fs::read_to_string(&fichier)?;
    let sauvegarde = analyser(&texte)?;

    if avec_donnees && sauvegarde.donnees.is_none() {
        return Err(ErreurApplication::Conflit(
            "ce fichier ne contient que le parametrage : decoche l import des donnees"
                .to_string(),
        ));
    }

    let donnees = if avec_donnees {
        sauvegarde.donnees.as_ref()
    } else {
        None
    };

    let resume = transfert::resumer_contenu(&sauvegarde.parametrage, donnees);

    depot.executer(|connexion| {
        let transaction = connexion.unchecked_transaction()?;

        if sessions::ouverte(&transaction)?.is_some() {
            return Err(ErreurApplication::Conflit(
                "une session de caisse est ouverte : la fermer avant d importer".to_string(),
            ));
        }

        // Sans import des donnees, les ventes de la caisse resteraient
        // accrochees a des comptes et a des produits qui disparaissent ici.
        if !avec_donnees {
            let presentes = transfert::resumer(&transaction)?;

            if presentes.ventes > 0 || presentes.sessions > 0 {
                return Err(ErreurApplication::Conflit(format!(
                    "{} vente(s) en memoire : edite un rapport Z (Parametrage --> Rapports) avant d importer le parametrage seul",
                    presentes.ventes
                )));
            }
        }

        // Ordre impose par les cles etrangeres, verifiees des l instruction :
        // l historique part d abord, les comptes et produits sont remplaces
        // ensuite, l historique importe arrive en dernier sur ces nouvelles
        // lignes.
        if donnees.is_some() {
            transfert::effacer_historique(&transaction)?;
        }

        transfert::ecrire_parametrage(&transaction, &sauvegarde.parametrage)?;

        if let Some(historique) = donnees {
            transfert::ecrire_donnees(&transaction, historique)?;
        }

        transaction.commit()?;

        Ok(())
    })?;

    Ok(ResultatTransfert {
        chemin: fichier.to_string_lossy().to_string(),
        nom: nom_lisible(&fichier),
        octets,
        contenu: sauvegarde.contenu,
        resume,
    })
}

fn portee(avec_donnees: bool) -> &'static str {
    if avec_donnees {
        CONTENU_COMPLET
    } else {
        CONTENU_PARAMETRAGE
    }
}

fn analyser(texte: &str) -> Resultat<Sauvegarde> {
    let sauvegarde: Sauvegarde = serde_json::from_str(texte).map_err(|erreur| {
        ErreurApplication::Validation(format!(
            "fichier illisible : ce n est pas une sauvegarde LCOS ({erreur})"
        ))
    })?;

    if sauvegarde.logiciel != LOGICIEL {
        return Err(ErreurApplication::Validation(format!(
            "fichier etranger a LCOS : logiciel {}",
            sauvegarde.logiciel
        )));
    }

    if sauvegarde.format != FORMAT {
        return Err(ErreurApplication::Validation(format!(
            "format de fichier {} inconnu : cette version lit le format {FORMAT}",
            sauvegarde.format
        )));
    }

    Ok(sauvegarde)
}

/// `Documents\LCOS` du compte Windows : un dossier que le gerant retrouve sans
/// explication, et ou l ecriture ne demande aucun privilege.
fn dossier_par_defaut() -> PathBuf {
    match std::env::var("USERPROFILE") {
        Ok(profil) if !profil.trim().is_empty() => {
            Path::new(&profil).join("Documents").join(DOSSIER_SAUVEGARDES)
        }
        _ => std::env::temp_dir().join(DOSSIER_SAUVEGARDES),
    }
}

fn resoudre_dossier(dossier: Option<String>) -> Resultat<PathBuf> {
    let Some(brut) = dossier else {
        return Ok(dossier_par_defaut());
    };

    let nettoye = brut.trim();

    if nettoye.is_empty() {
        return Ok(dossier_par_defaut());
    }

    let chemin = Path::new(nettoye);

    if !chemin.is_absolute() {
        return Err(ErreurApplication::Validation(
            "dossier : chemin complet attendu, par exemple E:\\ ou C:\\Users\\moi\\Documents"
                .to_string(),
        ));
    }

    Ok(chemin.to_path_buf())
}

fn valider_chemin(chemin: &str) -> Resultat<PathBuf> {
    let nettoye = chemin.trim();

    if nettoye.is_empty() {
        return Err(ErreurApplication::Validation(
            "aucun fichier choisi".to_string(),
        ));
    }

    let fichier = Path::new(nettoye);

    if !fichier.is_absolute() {
        return Err(ErreurApplication::Validation(
            "fichier : chemin complet attendu".to_string(),
        ));
    }

    if !fichier.is_file() {
        return Err(ErreurApplication::Introuvable(format!(
            "fichier absent : {nettoye}"
        )));
    }

    if !est_un_json(fichier) {
        return Err(ErreurApplication::Validation(
            "fichier : extension .json attendue".to_string(),
        ));
    }

    let taille = std::fs::metadata(fichier)?.len();

    if taille > TAILLE_MAXIMALE {
        return Err(ErreurApplication::Validation(format!(
            "fichier trop gros : maximum {} Mo",
            TAILLE_MAXIMALE / (1024 * 1024)
        )));
    }

    Ok(fichier.to_path_buf())
}

fn est_un_json(chemin: &Path) -> bool {
    chemin
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case(EXTENSION))
        .unwrap_or(false)
}

fn nom_lisible(chemin: &Path) -> String {
    chemin
        .file_name()
        .and_then(|nom| nom.to_str())
        .unwrap_or("sauvegarde.json")
        .to_string()
}

/// `lcos_parametrage_20261024-1742.json` : le nom porte la portee et l instant,
/// donc le tri alphabetique du dossier est deja un tri chronologique.
fn nom_fichier(contenu: &str, genere_le: &str) -> String {
    let chiffres: String = genere_le.chars().filter(|c| c.is_ascii_digit()).collect();
    let date = chiffres.get(0..8).unwrap_or("00000000");
    let heure = chiffres.get(8..12).unwrap_or("0000");

    format!("lcos_{contenu}_{date}-{heure}.{EXTENSION}")
}

/// Liste du dossier, les plus recents d abord. Un fichier illisible n est pas
/// cache : il est marque inutilisable, sinon le gerant le cherche en vain.
fn lister(dossier: &Path) -> Vec<FichierSauvegarde> {
    let Ok(entrees) = std::fs::read_dir(dossier) else {
        return Vec::new();
    };

    let mut fichiers: Vec<FichierSauvegarde> = Vec::new();

    for entree in entrees.flatten() {
        let chemin = entree.path();

        if !chemin.is_file() || !est_un_json(&chemin) {
            continue;
        }

        let octets = entree.metadata().map(|donnees| donnees.len()).unwrap_or(0);
        let entete = lire_entete(&chemin, octets);

        fichiers.push(FichierSauvegarde {
            nom: nom_lisible(&chemin),
            chemin: chemin.to_string_lossy().to_string(),
            octets,
            utilisable: entete.logiciel == LOGICIEL && entete.format == FORMAT,
            contenu: if entete.contenu.is_empty() {
                "inconnu".to_string()
            } else {
                entete.contenu
            },
            genere_le: entete.genere_le,
            version_logiciel: entete.version_logiciel,
        });
    }

    fichiers.sort_by(|gauche, droite| droite.nom.cmp(&gauche.nom));
    fichiers.truncate(FICHIERS_LISTES);

    fichiers
}

fn lire_entete(chemin: &Path, octets: u64) -> Entete {
    if octets > TAILLE_MAXIMALE {
        return Entete::default();
    }

    std::fs::read_to_string(chemin)
        .ok()
        .and_then(|texte| serde_json::from_str::<Entete>(&texte).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parametrage_minimal() -> transfert::Parametrage {
        transfert::Parametrage {
            parametres: vec![transfert::Parametre {
                cle: "nom_commerce".to_string(),
                valeur: "Chez Ali".to_string(),
            }],
            comptes: Vec::new(),
            rayons: Vec::new(),
            produits: Vec::new(),
        }
    }

    fn sauvegarde_minimale() -> Sauvegarde {
        Sauvegarde {
            logiciel: LOGICIEL.to_string(),
            format: FORMAT,
            version_logiciel: "1.4.0".to_string(),
            genere_le: "2026-10-24 17:42:05".to_string(),
            contenu: CONTENU_PARAMETRAGE.to_string(),
            parametrage: parametrage_minimal(),
            donnees: None,
        }
    }

    #[test]
    fn nomme_le_fichier_par_sa_portee_et_son_instant() {
        assert_eq!(
            nom_fichier(CONTENU_PARAMETRAGE, "2026-10-24 17:42:05"),
            "lcos_parametrage_20261024-1742.json"
        );
        assert_eq!(
            nom_fichier(CONTENU_COMPLET, "2026-01-02 08:05:00"),
            "lcos_complet_20260102-0805.json"
        );
    }

    #[test]
    fn nomme_sans_horodatage_lisible() {
        assert_eq!(
            nom_fichier(CONTENU_PARAMETRAGE, ""),
            "lcos_parametrage_00000000-0000.json"
        );
    }

    #[test]
    fn relit_ce_qu_il_a_ecrit() {
        let texte = serde_json::to_string_pretty(&sauvegarde_minimale()).unwrap();
        let relue = analyser(&texte).unwrap();

        assert_eq!(relue.contenu, CONTENU_PARAMETRAGE);
        assert_eq!(relue.parametrage.parametres[0].valeur, "Chez Ali");
        assert!(relue.donnees.is_none());
    }

    #[test]
    fn un_export_de_parametrage_ne_porte_pas_la_cle_donnees() {
        let texte = serde_json::to_string_pretty(&sauvegarde_minimale()).unwrap();

        assert!(!texte.contains("donnees"));
        assert!(texte.contains("\"contenu\": \"parametrage\""));
    }

    #[test]
    fn refuse_un_fichier_etranger() {
        assert!(analyser("{}").is_err());
        assert!(analyser("pas du json").is_err());

        let mut autre = sauvegarde_minimale();

        autre.logiciel = "AutreCaisse".to_string();
        assert!(analyser(&serde_json::to_string(&autre).unwrap()).is_err());
    }

    #[test]
    fn refuse_un_format_plus_recent() {
        let mut futur = sauvegarde_minimale();

        futur.format = FORMAT + 1;

        let incident = analyser(&serde_json::to_string(&futur).unwrap()).unwrap_err();

        assert!(incident.to_string().contains("format de fichier"));
    }

    #[test]
    fn exige_un_dossier_absolu() {
        assert!(resoudre_dossier(Some("sauvegardes".to_string())).is_err());
        assert!(resoudre_dossier(Some("C:\\Temp".to_string())).is_ok());
        assert_eq!(resoudre_dossier(None).unwrap(), dossier_par_defaut());
        assert_eq!(
            resoudre_dossier(Some("   ".to_string())).unwrap(),
            dossier_par_defaut()
        );
    }

    #[test]
    fn refuse_un_chemin_de_fichier_douteux() {
        assert!(valider_chemin("").is_err());
        assert!(valider_chemin("sauvegarde.json").is_err());
        assert!(valider_chemin("C:\\Temp\\absent_de_ce_poste.json").is_err());
    }

    #[test]
    fn ne_retient_que_l_extension_json() {
        assert!(est_un_json(Path::new("C:\\x\\lcos.json")));
        assert!(est_un_json(Path::new("C:\\x\\lcos.JSON")));
        assert!(!est_un_json(Path::new("C:\\x\\lcos.txt")));
        assert!(!est_un_json(Path::new("C:\\x\\lcos")));
    }
}
