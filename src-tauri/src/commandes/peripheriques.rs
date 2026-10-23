use serde::{Deserialize, Serialize};
use tauri::State;

use crate::depot::{parametres, Depot};
use crate::domaine::code_barres::{self, AnalyseCodeBarres};
use crate::erreur::{ErreurApplication, Resultat};

const CLE_ACTIF: &str = "scanner_actif";
const CLE_DELAI_MAX: &str = "scanner_delai_max_entre_touches";
const CLE_LONGUEUR_MIN: &str = "scanner_longueur_minimale";
const CLE_INACTIVITE: &str = "scanner_delai_inactivite";
const CLE_DETECTE: &str = "scanner_detecte";
const CLE_DERNIERE_DETECTION: &str = "scanner_derniere_detection";
const CLE_SIGNATURE: &str = "scanner_signature";

const BORNES_DELAI_MAX: (i64, i64) = (5, 500);
const BORNES_LONGUEUR_MIN: (i64, i64) = (1, 64);
const BORNES_INACTIVITE: (i64, i64) = (20, 2000);
const LONGUEUR_SIGNATURE_MAXIMALE: usize = 120;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReglagesScanner {
    pub actif: bool,
    pub delai_max_entre_touches: i64,
    pub longueur_minimale: i64,
    pub delai_inactivite: i64,
    pub detecte: bool,
    pub derniere_detection: String,
    pub signature: String,
}

#[tauri::command]
pub fn recuperer_reglages_scanner(depot: State<Depot>) -> Resultat<ReglagesScanner> {
    depot.executer(charger)
}

#[tauri::command]
pub fn enregistrer_reglages_scanner(
    depot: State<Depot>,
    reglages: ReglagesScanner,
) -> Resultat<ReglagesScanner> {
    let delai_max = borner(reglages.delai_max_entre_touches, BORNES_DELAI_MAX, "delai entre touches")?;
    let longueur_min = borner(reglages.longueur_minimale, BORNES_LONGUEUR_MIN, "longueur minimale")?;
    let inactivite = borner(reglages.delai_inactivite, BORNES_INACTIVITE, "delai d inactivite")?;

    depot.executer(|connexion| {
        parametres::ecrire(connexion, CLE_ACTIF, booleen_vers_texte(reglages.actif))?;
        parametres::ecrire(connexion, CLE_DELAI_MAX, &delai_max.to_string())?;
        parametres::ecrire(connexion, CLE_LONGUEUR_MIN, &longueur_min.to_string())?;
        parametres::ecrire(connexion, CLE_INACTIVITE, &inactivite.to_string())?;

        charger(connexion)
    })
}

/// Enregistre la douchette comme reconnue : signature materielle et horodatage.
#[tauri::command]
pub fn enregistrer_detection_scanner(
    depot: State<Depot>,
    signature: String,
) -> Resultat<ReglagesScanner> {
    let signature_nettoyee = signature.trim();

    if signature_nettoyee.is_empty() {
        return Err(ErreurApplication::Validation("signature vide".to_string()));
    }

    if signature_nettoyee.len() > LONGUEUR_SIGNATURE_MAXIMALE {
        return Err(ErreurApplication::Validation(format!(
            "signature trop longue: maximum {LONGUEUR_SIGNATURE_MAXIMALE} caracteres"
        )));
    }

    depot.executer(|connexion| {
        parametres::ecrire(connexion, CLE_DETECTE, "oui")?;
        parametres::ecrire(connexion, CLE_SIGNATURE, signature_nettoyee)?;
        parametres::horodater(connexion, CLE_DERNIERE_DETECTION)?;

        charger(connexion)
    })
}

#[tauri::command]
pub fn analyser_code_barres(code: String) -> Resultat<AnalyseCodeBarres> {
    code_barres::analyser(&code)
}

fn charger(connexion: &rusqlite::Connection) -> Resultat<ReglagesScanner> {
    let valeurs = parametres::lire_prefixe(connexion, "scanner_")?;

    let entier = |cle: &str, defaut: i64| -> i64 {
        valeurs
            .get(cle)
            .and_then(|valeur| valeur.parse::<i64>().ok())
            .unwrap_or(defaut)
    };

    let texte = |cle: &str| -> String { valeurs.get(cle).cloned().unwrap_or_default() };

    Ok(ReglagesScanner {
        actif: texte(CLE_ACTIF) == "oui",
        delai_max_entre_touches: entier(CLE_DELAI_MAX, 50),
        longueur_minimale: entier(CLE_LONGUEUR_MIN, 4),
        delai_inactivite: entier(CLE_INACTIVITE, 120),
        detecte: texte(CLE_DETECTE) == "oui",
        derniere_detection: texte(CLE_DERNIERE_DETECTION),
        signature: texte(CLE_SIGNATURE),
    })
}

fn borner(valeur: i64, bornes: (i64, i64), libelle: &str) -> Resultat<i64> {
    let (minimum, maximum) = bornes;

    if valeur < minimum || valeur > maximum {
        return Err(ErreurApplication::Validation(format!(
            "{libelle}: {valeur} hors bornes [{minimum}, {maximum}]"
        )));
    }

    Ok(valeur)
}

fn booleen_vers_texte(valeur: bool) -> &'static str {
    if valeur {
        "oui"
    } else {
        "non"
    }
}
