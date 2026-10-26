use tauri::State;

use crate::depot::{comptes, sessions, Depot};
use crate::domaine::secret;
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::{ROLE_CAISSIER, ROLE_GERANT};
use crate::securite::RegistreJetons;

use super::licence;

/// Liste visible sans code : l ecran d ouverture de session doit proposer les
/// caissiers. Aucune empreinte ne sort du depot.
#[tauri::command]
pub fn lister_caissiers_actifs(depot: State<Depot>) -> Resultat<Vec<comptes::Compte>> {
    depot.executer(|connexion| {
        let tous = comptes::lister(connexion, ROLE_CAISSIER)?;

        Ok(tous.into_iter().filter(|compte| compte.actif).collect())
    })
}

#[tauri::command]
pub fn lister_caissiers(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<Vec<comptes::Compte>> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(|connexion| comptes::lister(connexion, ROLE_CAISSIER))
}

#[tauri::command]
pub fn creer_caissier(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    nom: String,
    code: String,
) -> Resultat<comptes::Compte> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let nom_valide = comptes::valider_nom(&nom)?;
    let empreinte = secret::sceller(code.trim())?;

    depot.executer(|connexion| {
        let presents = comptes::lister(connexion, ROLE_CAISSIER)?.len() as i64;

        licence::exiger_quota_essai(connexion, "caissier(s)", presents, licence::ESSAI_CAISSIERS)?;

        let identifiant = comptes::creer(connexion, &nom_valide, ROLE_CAISSIER, &empreinte)?;

        comptes::exiger(connexion, identifiant)
    })
}

#[tauri::command]
pub fn modifier_caissier(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    identifiant: i64,
    nom: String,
    actif: bool,
) -> Resultat<comptes::Compte> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let nom_valide = comptes::valider_nom(&nom)?;

    depot.executer(|connexion| {
        let existant = comptes::exiger(connexion, identifiant)?;
        exiger_caissier(&existant)?;

        if !actif {
            refuser_si_session_ouverte(connexion, identifiant, "desactiver")?;
        }

        comptes::renommer(connexion, identifiant, &nom_valide)?;
        comptes::definir_actif(connexion, identifiant, actif)?;

        comptes::exiger(connexion, identifiant)
    })
}

#[tauri::command]
pub fn redefinir_code_caissier(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    identifiant: i64,
    code: String,
) -> Resultat<()> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let empreinte = secret::sceller(code.trim())?;

    depot.executer(|connexion| {
        let existant = comptes::exiger(connexion, identifiant)?;
        exiger_caissier(&existant)?;

        comptes::definir_secret(connexion, identifiant, &empreinte)
    })
}

#[tauri::command]
pub fn supprimer_caissier(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    identifiant: i64,
) -> Resultat<()> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(|connexion| {
        let existant = comptes::exiger(connexion, identifiant)?;
        exiger_caissier(&existant)?;
        refuser_si_session_ouverte(connexion, identifiant, "supprimer")?;

        comptes::supprimer(connexion, identifiant)
    })
}

fn exiger_caissier(compte: &comptes::Compte) -> Resultat<()> {
    if compte.role != ROLE_CAISSIER {
        return Err(ErreurApplication::AccesRefuse(
            "ce compte n est pas un caissier".to_string(),
        ));
    }

    Ok(())
}

fn refuser_si_session_ouverte(
    connexion: &rusqlite::Connection,
    identifiant: i64,
    action: &str,
) -> Resultat<()> {
    if let Some(session) = sessions::ouverte(connexion)? {
        if session.caissier == identifiant {
            return Err(ErreurApplication::Conflit(format!(
                "session de caisse ouverte : la fermer avant de {action} ce caissier"
            )));
        }
    }

    Ok(())
}
