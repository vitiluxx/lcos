/*
 * Date et heure du poste. Toutes les ventes sont horodatees par l horloge
 * systeme : une caisse a l heure fausse produit des rapports faux. Le reglage
 * passe par le systeme et demande donc les droits administrateur ; si LCOS ne
 * les a pas, le panneau Windows est ouvert pour que le gerant le fasse.
 */
use std::process::Command;

use serde::Serialize;
use tauri::State;

use crate::depot::{horloge, Depot};
use crate::domaine::horodatage;
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::RegistreJetons;

use super::administration;

const CREATION_SANS_FENETRE: u32 = 0x0800_0000;
const ANNEE_MINIMALE: i64 = 2000;
const ANNEE_MAXIMALE: i64 = 2099;

#[derive(Serialize)]
pub struct Horloge {
    pub brut: String,
    pub lisible: String,
    pub date: String,
    pub heure: String,
}

#[tauri::command]
pub fn recuperer_horloge(depot: State<Depot>) -> Resultat<Horloge> {
    depot.executer(|connexion| {
        let brut = horloge::maintenant(connexion)?;

        Ok(decomposer(&brut))
    })
}

#[tauri::command]
pub fn regler_horloge(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    date: String,
    heure: String,
) -> Resultat<Horloge> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let (annee, mois, jour) = valider_date(&date)?;
    let (heures, minutes) = valider_heure(&heure)?;

    let consigne = format!(
        "Set-Date -Date '{annee:04}-{mois:02}-{jour:02} {heures:02}:{minutes:02}:00'"
    );

    let sortie = lancer(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", &consigne],
    )?;

    // Sans droits administrateur, Set-Date est refuse : on redemande l horloge
    // dans un processus eleve plutot que d imposer une relance de LCOS.
    if !sortie.status.success() {
        administration::executer_eleve(&consigne).map_err(|incident| {
            let detail = String::from_utf8_lossy(&sortie.stderr);
            let premiere = detail.lines().next().unwrap_or("echec du reglage");

            ErreurApplication::AccesRefuse(format!(
                "reglage refuse par Windows : {premiere}. {incident}"
            ))
        })?;
    }

    recuperer_horloge(depot)
}

/// Repli lorsque LCOS n a pas les droits : le panneau Windows fait le travail.
#[tauri::command]
pub fn ouvrir_reglages_horloge(
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<()> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    lancer("cmd", &["/C", "start", "", "ms-settings:dateandtime"])?;

    Ok(())
}

fn decomposer(brut: &str) -> Horloge {
    let date = brut.get(0..10).unwrap_or_default().to_string();
    let heure = brut.get(11..16).unwrap_or_default().to_string();

    Horloge {
        brut: brut.to_string(),
        lisible: horodatage::lisible(brut),
        date,
        heure,
    }
}

fn lancer(programme: &str, arguments: &[&str]) -> Resultat<std::process::Output> {
    let mut commande = Command::new(programme);
    commande.args(arguments);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        commande.creation_flags(CREATION_SANS_FENETRE);
    }

    Ok(commande.output()?)
}

fn valider_date(date: &str) -> Resultat<(i64, i64, i64)> {
    let tranches: Vec<&str> = date.trim().split('-').collect();

    if tranches.len() != 3 {
        return Err(ErreurApplication::Validation(
            "date attendue au format AAAA-MM-JJ".to_string(),
        ));
    }

    let annee = nombre(tranches[0], ANNEE_MINIMALE, ANNEE_MAXIMALE, "annee")?;
    let mois = nombre(tranches[1], 1, 12, "mois")?;
    let jour = nombre(tranches[2], 1, jours_du_mois(annee, mois), "jour")?;

    Ok((annee, mois, jour))
}

fn valider_heure(heure: &str) -> Resultat<(i64, i64)> {
    let tranches: Vec<&str> = heure.trim().split(':').collect();

    if tranches.len() < 2 {
        return Err(ErreurApplication::Validation(
            "heure attendue au format HH:MM".to_string(),
        ));
    }

    let heures = nombre(tranches[0], 0, 23, "heures")?;
    let minutes = nombre(tranches[1], 0, 59, "minutes")?;

    Ok((heures, minutes))
}

fn nombre(tranche: &str, minimum: i64, maximum: i64, libelle: &str) -> Resultat<i64> {
    if tranche.is_empty() || !tranche.chars().all(|caractere| caractere.is_ascii_digit()) {
        return Err(ErreurApplication::Validation(format!(
            "{libelle} : chiffres attendus"
        )));
    }

    let valeur: i64 = tranche
        .parse()
        .map_err(|_| ErreurApplication::Validation(format!("{libelle} illisible")))?;

    if valeur < minimum || valeur > maximum {
        return Err(ErreurApplication::Validation(format!(
            "{libelle} : de {minimum} a {maximum}"
        )));
    }

    Ok(valeur)
}

fn jours_du_mois(annee: i64, mois: i64) -> i64 {
    match mois {
        2 if annee_bissextile(annee) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn annee_bissextile(annee: i64) -> bool {
    (annee % 4 == 0 && annee % 100 != 0) || annee % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valide_une_date_correcte() {
        assert_eq!(valider_date("2026-10-24").unwrap(), (2026, 10, 24));
        assert_eq!(valider_heure("01:12").unwrap(), (1, 12));
    }

    #[test]
    fn refuse_une_date_impossible() {
        assert!(valider_date("2026-02-30").is_err());
        assert!(valider_date("2026-13-01").is_err());
        assert!(valider_date("1999-01-01").is_err());
        assert!(valider_date("2026/10/24").is_err());
        assert!(valider_heure("24:00").is_err());
        assert!(valider_heure("12").is_err());
    }

    #[test]
    fn connait_les_annees_bissextiles() {
        assert_eq!(jours_du_mois(2024, 2), 29);
        assert_eq!(jours_du_mois(2026, 2), 28);
        assert_eq!(jours_du_mois(2000, 2), 29);
        assert_eq!(jours_du_mois(1900, 2), 28);
    }
}
