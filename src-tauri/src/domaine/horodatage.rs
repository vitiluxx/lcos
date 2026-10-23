/*
 * SQLite stocke les dates en "AAAA-MM-JJ HH:MM:SS" heure locale. L affichage
 * et le papier veulent le format francais. Pas de bibliotheque de dates : un
 * decoupage de positions suffit et reste verifiable.
 */
pub const ABSENT: &str = "--";

/// "2026-10-24 01:12:33" -> "24/10/2026 01:12"
pub fn lisible(brut: &str) -> String {
    match decouper(brut) {
        Some((annee, mois, jour, heure, minute)) => {
            format!("{jour}/{mois}/{annee} {heure}:{minute}")
        }
        None => ABSENT.to_string(),
    }
}

/// "2026-10-24 01:12:33" -> "24/10 01:12", pour les colonnes etroites du papier.
pub fn abreger(brut: &str) -> String {
    match decouper(brut) {
        Some((_, mois, jour, heure, minute)) => format!("{jour}/{mois} {heure}:{minute}"),
        None => ABSENT.to_string(),
    }
}

fn decouper(brut: &str) -> Option<(&str, &str, &str, &str, &str)> {
    let nettoye = brut.trim();

    if nettoye.len() < 16 {
        return None;
    }

    let annee = nettoye.get(0..4)?;
    let mois = nettoye.get(5..7)?;
    let jour = nettoye.get(8..10)?;
    let heure = nettoye.get(11..13)?;
    let minute = nettoye.get(14..16)?;

    if ![annee, mois, jour, heure, minute]
        .iter()
        .all(|tranche| tranche.chars().all(|caractere| caractere.is_ascii_digit()))
    {
        return None;
    }

    Some((annee, mois, jour, heure, minute))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn met_en_forme_une_date_sqlite() {
        assert_eq!(lisible("2026-10-24 01:12:33"), "24/10/2026 01:12");
        assert_eq!(abreger("2026-10-24 01:12:33"), "24/10 01:12");
    }

    #[test]
    fn tolere_une_valeur_absente() {
        assert_eq!(lisible(""), ABSENT);
        assert_eq!(lisible("jamais"), ABSENT);
        assert_eq!(abreger("2026-10-24"), ABSENT);
    }
}
