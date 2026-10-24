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

/// Jours ecoules depuis le 1970-01-01 pour un horodatage SQLite. Sert a
/// compter les jours d un essai gratuit ou la validite d une licence, sans
/// bibliotheque de dates : l algorithme civil tient en quinze lignes et se
/// verifie par des dates connues.
pub fn jours_depuis_epoque(brut: &str) -> Option<i64> {
    let (annee, mois, jour) = decouper_date(brut)?;

    let annee_decalee = if mois <= 2 { annee - 1 } else { annee };
    let ere = if annee_decalee >= 0 {
        annee_decalee
    } else {
        annee_decalee - 399
    } / 400;
    let annee_de_ere = annee_decalee - ere * 400;
    let mois_decale = if mois > 2 { mois - 3 } else { mois + 9 };
    let jour_de_annee = (153 * mois_decale + 2) / 5 + jour - 1;
    let jour_de_ere =
        annee_de_ere * 365 + annee_de_ere / 4 - annee_de_ere / 100 + jour_de_annee;

    Some(ere * 146_097 + jour_de_ere - 719_468)
}

/// Nombre de jours entiers entre deux horodatages, negatif si la fin precede
/// le debut.
pub fn jours_entre(debut: &str, fin: &str) -> Option<i64> {
    Some(jours_depuis_epoque(fin)? - jours_depuis_epoque(debut)?)
}

/// Operation inverse : une date "AAAA-MM-JJ" depuis un nombre de jours. Sert a
/// datter une licence et a calculer sa fin de validite.
pub fn date_depuis_epoque(jours: i64) -> String {
    let decale = jours + 719_468;
    let ere = if decale >= 0 { decale } else { decale - 146_096 } / 146_097;
    let jour_de_ere = decale - ere * 146_097;
    let annee_de_ere = (jour_de_ere - jour_de_ere / 1460 + jour_de_ere / 36_524
        - jour_de_ere / 146_096)
        / 365;
    let jour_de_annee =
        jour_de_ere - (365 * annee_de_ere + annee_de_ere / 4 - annee_de_ere / 100);
    let mois_decale = (5 * jour_de_annee + 2) / 153;
    let jour = jour_de_annee - (153 * mois_decale + 2) / 5 + 1;
    let mois = if mois_decale < 10 {
        mois_decale + 3
    } else {
        mois_decale - 9
    };
    let annee = annee_de_ere + ere * 400 + i64::from(mois <= 2);

    format!("{annee:04}-{mois:02}-{jour:02}")
}

/// Date du jour selon l horloge du systeme, en temps universel. L emission
/// d une licence n a pas besoin de plus fin que le jour.
pub fn aujourd_hui() -> String {
    let secondes = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|ecart| ecart.as_secs() as i64)
        .unwrap_or(0);

    date_depuis_epoque(secondes / 86_400)
}

fn decouper_date(brut: &str) -> Option<(i64, i64, i64)> {
    let nettoye = brut.trim();

    if nettoye.len() < 10 {
        return None;
    }

    let annee: i64 = nettoye.get(0..4)?.parse().ok()?;
    let mois: i64 = nettoye.get(5..7)?.parse().ok()?;
    let jour: i64 = nettoye.get(8..10)?.parse().ok()?;

    if !(1..=12).contains(&mois) || !(1..=31).contains(&jour) {
        return None;
    }

    Some((annee, mois, jour))
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
    fn compte_les_jours_depuis_l_epoque() {
        assert_eq!(jours_depuis_epoque("1970-01-01").unwrap(), 0);
        assert_eq!(jours_depuis_epoque("1970-01-02 23:59:59").unwrap(), 1);
        assert_eq!(jours_depuis_epoque("2000-03-01").unwrap(), 11_017);
        assert_eq!(jours_depuis_epoque("2026-10-24 18:03:27").unwrap(), 20_750);
        assert_eq!(jours_depuis_epoque("1969-12-31").unwrap(), -1);
    }

    #[test]
    fn refait_la_date_depuis_les_jours() {
        assert_eq!(date_depuis_epoque(0), "1970-01-01");
        assert_eq!(date_depuis_epoque(20_750), "2026-10-24");
        assert_eq!(date_depuis_epoque(11_017), "2000-03-01");
        assert_eq!(date_depuis_epoque(-1), "1969-12-31");

        for jours in [0, 1, 999, 11_017, 20_750, 25_000, 40_000] {
            assert_eq!(
                jours_depuis_epoque(&date_depuis_epoque(jours)).unwrap(),
                jours
            );
        }
    }

    #[test]
    fn la_date_du_jour_est_lisible() {
        let aujourd_hui = aujourd_hui();

        assert_eq!(aujourd_hui.len(), 10);
        assert!(jours_depuis_epoque(&aujourd_hui).unwrap() > 20_000);
    }

    #[test]
    fn compte_les_jours_entre_deux_dates() {
        assert_eq!(jours_entre("2026-10-24", "2026-10-31").unwrap(), 7);
        assert_eq!(jours_entre("2026-02-27", "2026-03-01").unwrap(), 2);
        assert_eq!(jours_entre("2024-02-27", "2024-03-01").unwrap(), 3);
        assert_eq!(jours_entre("2026-10-31", "2026-10-24").unwrap(), -7);
    }

    #[test]
    fn refuse_une_date_illisible() {
        assert!(jours_depuis_epoque("").is_none());
        assert!(jours_depuis_epoque("24/10/2026").is_none());
        assert!(jours_depuis_epoque("2026-13-01").is_none());
        assert!(jours_depuis_epoque("2026-10-00").is_none());
    }

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
