use serde::Serialize;
use tauri::State;

use crate::depot::{catalogue, Depot};
use crate::domaine::{code_barres, monnaie};
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::RegistreJetons;

use super::licence;

const ORDRE_MAXIMAL: i64 = 999;
const STOCK_MAXIMAL: i64 = 1_000_000;

/// Valeur du champ stock qui dispense le produit de tout decompte. C est la
/// valeur par defaut d une fiche neuve : on vend d abord, on compte apres.
const STOCK_SANS_SUIVI: &str = "NON";

#[derive(Serialize)]
pub struct ResultatRecherche {
    pub trouve: bool,
    pub produit: Option<catalogue::Produit>,
    pub code: String,
    pub symbologie: String,
}

#[tauri::command]
pub fn lister_rayons(
    depot: State<Depot>,
    inclure_inactifs: Option<bool>,
) -> Resultat<Vec<catalogue::Rayon>> {
    depot.executer(|connexion| {
        catalogue::lister_rayons(connexion, inclure_inactifs.unwrap_or(false))
    })
}

#[tauri::command]
pub fn creer_rayon(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    libelle: String,
    ordre: Option<i64>,
) -> Resultat<catalogue::Rayon> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let libelle_valide = catalogue::valider_libelle(&libelle)?;
    let ordre_valide = borner_ordre(ordre.unwrap_or(0))?;

    depot.executer(|connexion| {
        let presents = catalogue::lister_rayons(connexion, true)?.len() as i64;

        licence::exiger_quota_essai(connexion, "rayon(s)", presents, licence::ESSAI_RAYONS)?;

        let identifiant = catalogue::creer_rayon(connexion, &libelle_valide, ordre_valide)?;

        catalogue::exiger_rayon(connexion, identifiant)
    })
}

#[tauri::command]
pub fn modifier_rayon(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    identifiant: i64,
    libelle: String,
    ordre: Option<i64>,
    actif: bool,
) -> Resultat<catalogue::Rayon> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let libelle_valide = catalogue::valider_libelle(&libelle)?;
    let ordre_valide = borner_ordre(ordre.unwrap_or(0))?;

    depot.executer(|connexion| {
        catalogue::modifier_rayon(
            connexion,
            identifiant,
            &libelle_valide,
            ordre_valide,
            actif,
        )?;

        catalogue::exiger_rayon(connexion, identifiant)
    })
}

#[tauri::command]
pub fn supprimer_rayon(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    identifiant: i64,
) -> Resultat<()> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(|connexion| catalogue::supprimer_rayon(connexion, identifiant))
}

#[tauri::command]
pub fn lister_produits(
    depot: State<Depot>,
    recherche: Option<String>,
    rayon: Option<i64>,
    inclure_inactifs: Option<bool>,
) -> Resultat<Vec<catalogue::Produit>> {
    let filtre = catalogue::Filtre {
        recherche: recherche.filter(|texte| !texte.trim().is_empty()),
        rayon,
        inclure_inactifs: inclure_inactifs.unwrap_or(false),
    };

    depot.executer(|connexion| catalogue::lister_produits(connexion, &filtre))
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn creer_produit(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    rayon: i64,
    libelle: String,
    code_barres: Option<String>,
    prix: String,
    stock: Option<String>,
) -> Resultat<catalogue::Produit> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let libelle_valide = catalogue::valider_libelle(&libelle)?;
    let prix_centimes = monnaie::analyser(&prix)?;
    let code = normaliser_code(code_barres)?;
    let (stock_valide, suivi) = analyser_stock(stock)?;

    depot.executer(|connexion| {
        catalogue::exiger_rayon(connexion, rayon)?;

        let presents = catalogue::compter_produits(connexion)?;

        licence::exiger_quota_essai(connexion, "produits", presents, licence::ESSAI_PRODUITS)?;

        let identifiant = catalogue::creer_produit(
            connexion,
            rayon,
            &libelle_valide,
            code.as_deref(),
            prix_centimes,
            stock_valide,
            suivi,
        )?;

        catalogue::exiger_produit(connexion, identifiant)
    })
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn modifier_produit(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    identifiant: i64,
    rayon: i64,
    libelle: String,
    code_barres: Option<String>,
    prix: String,
    stock: Option<String>,
    actif: bool,
) -> Resultat<catalogue::Produit> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let libelle_valide = catalogue::valider_libelle(&libelle)?;
    let prix_centimes = monnaie::analyser(&prix)?;
    let code = normaliser_code(code_barres)?;
    let (stock_valide, suivi) = analyser_stock(stock)?;

    depot.executer(|connexion| {
        catalogue::exiger_rayon(connexion, rayon)?;
        catalogue::modifier_produit(
            connexion,
            identifiant,
            rayon,
            &libelle_valide,
            code.as_deref(),
            prix_centimes,
            stock_valide,
            suivi,
            actif,
        )?;

        catalogue::exiger_produit(connexion, identifiant)
    })
}

#[tauri::command]
pub fn supprimer_produit(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    identifiant: i64,
) -> Resultat<()> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(|connexion| catalogue::supprimer_produit(connexion, identifiant))
}

/// Chemin du scan : un code lu par la douchette arrive ici.
#[tauri::command]
pub fn trouver_produit_par_code(depot: State<Depot>, code: String) -> Resultat<ResultatRecherche> {
    let analyse = code_barres::analyser(&code)?;

    depot.executer(|connexion| {
        let produit = catalogue::trouver_par_code(connexion, &analyse.code)?;

        Ok(ResultatRecherche {
            trouve: produit.is_some(),
            produit,
            code: analyse.code.clone(),
            symbologie: analyse.symbologie.clone(),
        })
    })
}

fn borner_ordre(ordre: i64) -> Resultat<i64> {
    if !(0..=ORDRE_MAXIMAL).contains(&ordre) {
        return Err(ErreurApplication::Validation(format!(
            "ordre : de 0 a {ORDRE_MAXIMAL}"
        )));
    }

    Ok(ordre)
}

/// Le champ stock de la fiche produit accepte deux formes : un nombre, ou
/// NON. NON se vend sans decompte : aucun stock n est exige a la vente et
/// aucune quantite n est retiree a l encaissement.
fn analyser_stock(saisie: Option<String>) -> Resultat<(i64, bool)> {
    let brut = saisie.unwrap_or_default();
    let nettoye = brut.trim();

    if nettoye.is_empty() || nettoye.eq_ignore_ascii_case(STOCK_SANS_SUIVI) {
        return Ok((0, false));
    }

    let quantite: i64 = nettoye.parse().map_err(|_| {
        ErreurApplication::Validation(format!(
            "stock : un nombre entier, ou {STOCK_SANS_SUIVI} pour vendre sans decompte"
        ))
    })?;

    Ok((borner_stock(quantite)?, true))
}

fn borner_stock(stock: i64) -> Resultat<i64> {
    if !(0..=STOCK_MAXIMAL).contains(&stock) {
        return Err(ErreurApplication::Validation(format!(
            "stock : de 0 a {STOCK_MAXIMAL}"
        )));
    }

    Ok(stock)
}

/// Un code-barres vide vaut absence de code : le produit se vend alors au
/// toucher depuis la grille des rayons.
fn normaliser_code(code_barres: Option<String>) -> Resultat<Option<String>> {
    match code_barres {
        None => Ok(None),
        Some(brut) if brut.trim().is_empty() => Ok(None),
        Some(brut) => {
            let analyse = code_barres::analyser(&brut)?;

            Ok(Some(analyse.code))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_stock_absent_ou_non_dispense_du_suivi() {
        assert_eq!(analyser_stock(None).unwrap(), (0, false));
        assert_eq!(analyser_stock(Some(String::new())).unwrap(), (0, false));
        assert_eq!(analyser_stock(Some("NON".to_string())).unwrap(), (0, false));
        assert_eq!(analyser_stock(Some("  non  ".to_string())).unwrap(), (0, false));
    }

    #[test]
    fn un_nombre_active_le_suivi() {
        assert_eq!(analyser_stock(Some("12".to_string())).unwrap(), (12, true));
        assert_eq!(analyser_stock(Some(" 0 ".to_string())).unwrap(), (0, true));
    }

    #[test]
    fn refuse_une_saisie_de_stock_illisible() {
        assert!(analyser_stock(Some("douze".to_string())).is_err());
        assert!(analyser_stock(Some("-3".to_string())).is_err());
        assert!(analyser_stock(Some("9999999999".to_string())).is_err());
    }
}
