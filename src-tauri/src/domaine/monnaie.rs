/*
 * Toute somme circule en centimes entiers : aucun flottant dans la chaine de
 * calcul. Le formatage est fait ici pour que le ticket papier et l'ecran
 * affichent exactement la meme chose.
 */
use crate::erreur::{ErreurApplication, Resultat};

pub const MONTANT_MAXIMAL: i64 = 99_999_999;

/// Decimales affichees par defaut : deux, comme l euro.
pub const DECIMALES_DEFAUT: i64 = 2;

/// Les devises sans subdivision (FCFA, XOF, XAF) s affichent en entiers
/// pleins : une virgule a l ecran d une caisse en franc CFA est une erreur de
/// lecture, pas un detail de presentation.
pub fn borner_decimales(decimales: i64) -> i64 {
    if decimales <= 0 {
        0
    } else {
        DECIMALES_DEFAUT
    }
}

pub fn valider(centimes: i64) -> Resultat<i64> {
    if centimes < 0 {
        return Err(ErreurApplication::Validation(
            "montant negatif refuse".to_string(),
        ));
    }

    if centimes > MONTANT_MAXIMAL {
        return Err(ErreurApplication::Validation(
            "montant hors limite de la caisse".to_string(),
        ));
    }

    Ok(centimes)
}

/// Lit une saisie humaine : "15", "15,5", "15.50", "1 500,00" -> centimes.
pub fn analyser(saisie: &str) -> Resultat<i64> {
    let nettoyee: String = saisie
        .chars()
        .filter(|caractere| !caractere.is_whitespace())
        .collect();

    if nettoyee.is_empty() {
        return Err(ErreurApplication::Validation("montant vide".to_string()));
    }

    let normalisee = nettoyee.replace(',', ".");
    let mut tranches = normalisee.split('.');

    let entiere = tranches.next().unwrap_or_default();
    let decimale = tranches.next().unwrap_or("0");

    if tranches.next().is_some() {
        return Err(ErreurApplication::Validation(
            "montant mal forme : un seul separateur decimal".to_string(),
        ));
    }

    if entiere.is_empty() || !entiere.chars().all(|caractere| caractere.is_ascii_digit()) {
        return Err(ErreurApplication::Validation(format!(
            "montant invalide : {saisie}"
        )));
    }

    if decimale.len() > 2 || !decimale.chars().all(|caractere| caractere.is_ascii_digit()) {
        return Err(ErreurApplication::Validation(
            "montant invalide : deux decimales au maximum".to_string(),
        ));
    }

    let unites: i64 = entiere
        .parse()
        .map_err(|_| ErreurApplication::Validation("montant trop grand".to_string()))?;

    let centimes_decimaux: i64 = match decimale.len() {
        0 => 0,
        1 => decimale.parse::<i64>().unwrap_or(0) * 10,
        _ => decimale.parse::<i64>().unwrap_or(0),
    };

    let total = unites
        .checked_mul(100)
        .and_then(|valeur| valeur.checked_add(centimes_decimaux))
        .ok_or_else(|| ErreurApplication::Validation("montant trop grand".to_string()))?;

    valider(total)
}

/// Rend "1 234,50" : groupes de trois, virgule decimale, deux decimales.
pub fn formater(centimes: i64) -> String {
    formater_avec(centimes, DECIMALES_DEFAUT)
}

/// Rend "1 234,50" a deux decimales, "1 234" a zero decimale. La monnaie
/// circule toujours en centimes : seule la presentation change.
pub fn formater_avec(centimes: i64, decimales: i64) -> String {
    let negatif = centimes < 0;
    let absolu = centimes.abs();
    let unites = absolu / 100;
    let reste = absolu % 100;

    let chiffres: Vec<char> = unites.to_string().chars().collect();
    let mut groupes = String::new();

    for (rang, chiffre) in chiffres.iter().enumerate() {
        if rang > 0 && (chiffres.len() - rang) % 3 == 0 {
            groupes.push(' ');
        }

        groupes.push(*chiffre);
    }

    let signe = if negatif { "-" } else { "" };

    if borner_decimales(decimales) == 0 {
        return format!("{signe}{groupes}");
    }

    format!("{signe}{groupes},{reste:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyse_les_formes_courantes() {
        assert_eq!(analyser("15").unwrap(), 1500);
        assert_eq!(analyser("15,5").unwrap(), 1550);
        assert_eq!(analyser("15.50").unwrap(), 1550);
        assert_eq!(analyser(" 1 500,00 ").unwrap(), 150_000);
        assert_eq!(analyser("0,05").unwrap(), 5);
    }

    #[test]
    fn refuse_les_saisies_douteuses() {
        assert!(analyser("").is_err());
        assert!(analyser("abc").is_err());
        assert!(analyser("1,234").is_err());
        assert!(analyser("1,2,3").is_err());
        assert!(analyser("-5").is_err());
    }

    #[test]
    fn formate_avec_groupes_de_trois() {
        assert_eq!(formater(0), "0,00");
        assert_eq!(formater(5), "0,05");
        assert_eq!(formater(1550), "15,50");
        assert_eq!(formater(150_000), "1 500,00");
        assert_eq!(formater(123_456_789), "1 234 567,89");
    }

    #[test]
    fn formate_sans_decimale_pour_le_franc_cfa() {
        assert_eq!(formater_avec(50_000, 0), "500");
        assert_eq!(formater_avec(0, 0), "0");
        assert_eq!(formater_avec(123_456_789, 0), "1 234 567");
        assert_eq!(formater_avec(-50_000, 0), "-500");
    }

    #[test]
    fn borne_le_nombre_de_decimales() {
        assert_eq!(borner_decimales(0), 0);
        assert_eq!(borner_decimales(-3), 0);
        assert_eq!(borner_decimales(2), 2);
        assert_eq!(borner_decimales(9), 2);
    }

    #[test]
    fn borne_les_montants() {
        assert!(valider(0).is_ok());
        assert!(valider(MONTANT_MAXIMAL).is_ok());
        assert!(valider(MONTANT_MAXIMAL + 1).is_err());
        assert!(valider(-1).is_err());
    }
}
