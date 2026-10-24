/*
 * Droits administrateur du poste. Regler l horloge Windows exige une
 * elevation. Plutot que d obliger le gerant a relancer LCOS par un clic droit
 * a chaque fois, on installe une tache planifiee au niveau le plus eleve et un
 * raccourci qui la declenche : Windows n affiche alors plus aucune demande au
 * demarrage. L installation de la tache demande elle-meme une elevation, une
 * seule fois.
 */
use std::process::Command;

use serde::Serialize;
use tauri::State;

use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::RegistreJetons;

const CREATION_SANS_FENETRE: u32 = 0x0800_0000;
const NOM_TACHE: &str = "LCOS_Administrateur";
const NOM_RACCOURCI: &str = "LCOS (administrateur).lnk";

#[derive(Debug, Serialize)]
pub struct EtatAdministrateur {
    pub eleve: bool,
    pub tache_installee: bool,
    pub programme: String,
    pub nom_tache: String,
    pub raccourci: String,
}

#[tauri::command]
pub fn recuperer_etat_administrateur() -> Resultat<EtatAdministrateur> {
    Ok(EtatAdministrateur {
        eleve: session_elevee(),
        tache_installee: tache_presente(),
        programme: programme()?,
        nom_tache: NOM_TACHE.to_string(),
        raccourci: raccourci()?,
    })
}

/// Installe la tache planifiee et le raccourci du bureau. Une seule demande
/// d elevation Windows, puis plus aucune.
#[tauri::command]
pub fn activer_lancement_administrateur(
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<EtatAdministrateur> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let exe = programme()?;
    let lien = raccourci()?;
    let utilisateur = utilisateur();

    let script = format!(
        "$action = New-ScheduledTaskAction -Execute '{exe}'; \
         $principal = New-ScheduledTaskPrincipal -UserId '{utilisateur}' \
             -LogonType Interactive -RunLevel Highest; \
         $reglages = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries \
             -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero); \
         Register-ScheduledTask -TaskName '{tache}' -Action $action \
             -Principal $principal -Settings $reglages -Force | Out-Null; \
         $shell = New-Object -ComObject WScript.Shell; \
         $lien = $shell.CreateShortcut('{lien}'); \
         $lien.TargetPath = [IO.Path]::Combine($env:SystemRoot, 'System32', 'schtasks.exe'); \
         $lien.Arguments = '/run /tn {tache}'; \
         $lien.IconLocation = '{exe},0'; \
         $lien.WindowStyle = 7; \
         $lien.Save()",
        exe = echapper(&exe),
        utilisateur = echapper(&utilisateur),
        tache = NOM_TACHE,
        lien = echapper(&lien),
    );

    executer_eleve(&script)?;

    recuperer_etat_administrateur()
}

#[tauri::command]
pub fn desactiver_lancement_administrateur(
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<EtatAdministrateur> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let lien = raccourci()?;

    let script = format!(
        "Unregister-ScheduledTask -TaskName '{tache}' -Confirm:$false \
             -ErrorAction SilentlyContinue; \
         Remove-Item -LiteralPath '{lien}' -Force -ErrorAction SilentlyContinue",
        tache = NOM_TACHE,
        lien = echapper(&lien),
    );

    executer_eleve(&script)?;

    recuperer_etat_administrateur()
}

/// Vrai si le processus courant tourne deja avec les droits administrateur.
pub fn session_elevee() -> bool {
    let consigne = "if (([Security.Principal.WindowsPrincipal] \
         [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole( \
         [Security.Principal.WindowsBuiltInRole]::Administrator)) { exit 0 } else { exit 1 }";

    lancer(&["-NoProfile", "-NonInteractive", "-Command", consigne])
        .map(|sortie| sortie.status.success())
        .unwrap_or(false)
}

/// Relance une consigne PowerShell dans un processus eleve. Windows affiche
/// sa demande d autorisation ; un refus remonte une erreur explicite.
pub fn executer_eleve(script: &str) -> Resultat<()> {
    let enveloppe = format!(
        "$processus = Start-Process powershell -Verb RunAs -WindowStyle Hidden -Wait \
             -PassThru -ArgumentList @('-NoProfile', '-NonInteractive', '-Command', '{}'); \
         exit $processus.ExitCode",
        echapper(script)
    );

    let sortie = lancer(&["-NoProfile", "-NonInteractive", "-Command", &enveloppe])?;

    if sortie.status.success() {
        return Ok(());
    }

    let detail = String::from_utf8_lossy(&sortie.stderr);
    let premiere = detail.lines().next().unwrap_or("").trim();

    Err(ErreurApplication::AccesRefuse(if premiere.is_empty() {
        "elevation refusee : Windows a annule la demande d autorisation".to_string()
    } else {
        format!("elevation refusee : {premiere}")
    }))
}

fn tache_presente() -> bool {
    lancer(&[
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &format!(
            "if (Get-ScheduledTask -TaskName '{NOM_TACHE}' -ErrorAction SilentlyContinue) \
             {{ exit 0 }} else {{ exit 1 }}"
        ),
    ])
    .map(|sortie| sortie.status.success())
    .unwrap_or(false)
}

fn programme() -> Resultat<String> {
    Ok(std::env::current_exe()?.to_string_lossy().to_string())
}

fn raccourci() -> Resultat<String> {
    let profil = std::env::var("USERPROFILE").map_err(|_| {
        ErreurApplication::Conflit("profil utilisateur Windows introuvable".to_string())
    })?;

    Ok(std::path::Path::new(&profil)
        .join("Desktop")
        .join(NOM_RACCOURCI)
        .to_string_lossy()
        .to_string())
}

fn utilisateur() -> String {
    let compte = std::env::var("USERNAME").unwrap_or_default();

    match std::env::var("USERDOMAIN") {
        Ok(domaine) if !domaine.is_empty() => format!("{domaine}\\{compte}"),
        _ => compte,
    }
}

/// Chaine PowerShell entre apostrophes : seule l apostrophe est a doubler.
fn echapper(valeur: &str) -> String {
    valeur.replace('\'', "''")
}

fn lancer(arguments: &[&str]) -> Resultat<std::process::Output> {
    let mut commande = Command::new("powershell");
    commande.args(arguments);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        commande.creation_flags(CREATION_SANS_FENETRE);
    }

    Ok(commande.output()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_les_apostrophes() {
        assert_eq!(echapper("d'accord"), "d''accord");
        assert_eq!(echapper("sans"), "sans");
    }

    #[test]
    fn compose_le_compte_windows() {
        // Le format attendu par New-ScheduledTaskPrincipal est DOMAINE\compte.
        let compose = utilisateur();

        assert!(!compose.contains('/'));
    }
}
