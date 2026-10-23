use serde::Serialize;

use crate::erreur::{ErreurApplication, Resultat};

pub const LONGUEUR_MAXIMALE: usize = 128;
pub const LONGUEUR_MINIMALE: usize = 1;

#[derive(Debug, Serialize, PartialEq)]
pub struct AnalyseCodeBarres {
    pub code: String,
    pub symbologie: String,
    pub longueur: usize,
    pub somme_controle_verifiable: bool,
    pub somme_controle_valide: bool,
}

/// Analyse un code lu par la douchette : nettoyage, controle de surface,
/// reconnaissance de la symbologie et verification de la cle de controle.
pub fn analyser(code_brut: &str) -> Resultat<AnalyseCodeBarres> {
    let code = code_brut.trim().to_string();

    if code.len() < LONGUEUR_MINIMALE {
        return Err(ErreurApplication::Validation("code vide".to_string()));
    }

    if code.len() > LONGUEUR_MAXIMALE {
        return Err(ErreurApplication::Validation(format!(
            "code trop long: {} caracteres, maximum {}",
            code.len(),
            LONGUEUR_MAXIMALE
        )));
    }

    if let Some(interdit) = code.chars().find(|caractere| !caractere_autorise(*caractere)) {
        return Err(ErreurApplication::Validation(format!(
            "caractere interdit: {:?}",
            interdit
        )));
    }

    let chiffres: Option<Vec<u32>> = code.chars().map(|caractere| caractere.to_digit(10)).collect();

    let (symbologie, verifiable, valide) = match chiffres {
        Some(valeurs) => reconnaitre_numerique(&valeurs),
        None => ("Alphanumerique".to_string(), false, false),
    };

    Ok(AnalyseCodeBarres {
        longueur: code.len(),
        code,
        symbologie,
        somme_controle_verifiable: verifiable,
        somme_controle_valide: valide,
    })
}

fn caractere_autorise(caractere: char) -> bool {
    caractere.is_ascii_graphic() || caractere == ' '
}

fn reconnaitre_numerique(chiffres: &[u32]) -> (String, bool, bool) {
    let nom = match chiffres.len() {
        13 => "EAN-13",
        12 => "UPC-A",
        8 => "EAN-8",
        _ => return ("Numerique".to_string(), false, false),
    };

    let valide = chiffres.last().copied() == Some(somme_controle(chiffres));

    (nom.to_string(), true, valide)
}

/// Cle de controle EAN/UPC : poids 3 et 1 alternes depuis le dernier chiffre de donnees.
fn somme_controle(chiffres: &[u32]) -> u32 {
    let donnees = &chiffres[..chiffres.len() - 1];

    let total: u32 = donnees
        .iter()
        .rev()
        .enumerate()
        .map(|(rang, chiffre)| chiffre * if rang % 2 == 0 { 3 } else { 1 })
        .sum();

    (10 - total % 10) % 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnait_un_ean13_valide() {
        let analyse = analyser("4006381333931").unwrap();

        assert_eq!(analyse.symbologie, "EAN-13");
        assert!(analyse.somme_controle_verifiable);
        assert!(analyse.somme_controle_valide);
    }

    #[test]
    fn detecte_une_cle_de_controle_fausse() {
        let analyse = analyser("4006381333930").unwrap();

        assert_eq!(analyse.symbologie, "EAN-13");
        assert!(!analyse.somme_controle_valide);
    }

    #[test]
    fn reconnait_ean8_et_upca() {
        assert_eq!(analyser("96385074").unwrap().symbologie, "EAN-8");
        assert!(analyser("96385074").unwrap().somme_controle_valide);

        assert_eq!(analyser("036000291452").unwrap().symbologie, "UPC-A");
        assert!(analyser("036000291452").unwrap().somme_controle_valide);
    }

    #[test]
    fn classe_un_code_alphanumerique() {
        let analyse = analyser("ART-2026-XY").unwrap();

        assert_eq!(analyse.symbologie, "Alphanumerique");
        assert!(!analyse.somme_controle_verifiable);
    }

    #[test]
    fn retire_les_espaces_de_bordure() {
        assert_eq!(analyser("  96385074  ").unwrap().code, "96385074");
    }

    #[test]
    fn refuse_le_vide_et_le_trop_long() {
        assert!(analyser("   ").is_err());
        assert!(analyser(&"9".repeat(LONGUEUR_MAXIMALE + 1)).is_err());
    }

    #[test]
    fn refuse_les_caracteres_de_controle() {
        assert!(analyser("123\u{0}456").is_err());
    }
}
