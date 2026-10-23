use serde::Serialize;
use tauri::State;

use crate::depot::{comptes, sessions, Depot};
use crate::domaine::{monnaie, secret};
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::{ROLE_CAISSIER, ROLE_GERANT};
use crate::securite::{Limiteur, RegistreJetons};

#[derive(Serialize)]
pub struct EtatSession {
    pub ouverte: bool,
    pub session: Option<sessions::SessionCaisse>,
}

#[tauri::command]
pub fn recuperer_session_courante(depot: State<Depot>) -> Resultat<EtatSession> {
    depot.executer(|connexion| {
        let session = sessions::ouverte(connexion)?;

        Ok(EtatSession {
            ouverte: session.is_some(),
            session,
        })
    })
}

#[tauri::command]
pub fn ouvrir_session_caissier(
    depot: State<Depot>,
    limiteur: State<Limiteur>,
    caissier: i64,
    code: String,
    fond: Option<String>,
) -> Resultat<sessions::SessionCaisse> {
    let cible = format!("caissier:{caissier}");
    limiteur.verifier(&cible)?;

    let fond_centimes = match fond {
        Some(saisie) if !saisie.trim().is_empty() => monnaie::analyser(&saisie)?,
        _ => 0,
    };

    let empreinte = depot
        .executer(|connexion| {
            let compte = comptes::exiger(connexion, caissier)?;

            if compte.role != ROLE_CAISSIER {
                return Err(ErreurApplication::AccesRefuse(
                    "ce compte n est pas un caissier".to_string(),
                ));
            }

            if !compte.actif {
                return Err(ErreurApplication::AccesRefuse(format!(
                    "le compte {} est desactive",
                    compte.nom
                )));
            }

            comptes::lire_empreinte(connexion, caissier)
        })?
        .ok_or_else(|| ErreurApplication::Introuvable("empreinte du caissier".to_string()))?;

    if !secret::verifier(code.trim(), &empreinte) {
        limiteur.echec(&cible);

        return Err(ErreurApplication::AccesRefuse("code incorrect".to_string()));
    }

    limiteur.reussite(&cible);

    depot.executer(|connexion| {
        let identifiant = sessions::ouvrir(connexion, caissier, fond_centimes)?;
        let session = sessions::ouverte(connexion)?;

        session.ok_or_else(|| {
            ErreurApplication::Introuvable(format!("session {identifiant} introuvable apres ouverture"))
        })
    })
}

/// Fermeture par le caissier lui-meme ou par le gerant avec son propre code.
#[tauri::command]
pub fn fermer_session_caissier(
    depot: State<Depot>,
    limiteur: State<Limiteur>,
    code: String,
) -> Resultat<sessions::SessionCaisse> {
    let session = depot.executer(sessions::exiger_ouverte)?;
    let cible = format!("fermeture:{}", session.identifiant);

    limiteur.verifier(&cible)?;

    let autorise = depot.executer(|connexion| {
        let empreinte_caissier = comptes::lire_empreinte(connexion, session.caissier)?;

        if let Some(empreinte) = empreinte_caissier {
            if secret::verifier(code.trim(), &empreinte) {
                return Ok(true);
            }
        }

        // Le gerant peut toujours fermer une session, caissier absent compris.
        if let Some(gerant) = comptes::lire_par_role(connexion, ROLE_GERANT)? {
            if let Some(empreinte) = comptes::lire_empreinte(connexion, gerant.identifiant)? {
                if secret::verifier(code.trim(), &empreinte) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    })?;

    if !autorise {
        limiteur.echec(&cible);

        return Err(ErreurApplication::AccesRefuse(
            "code du caissier ou du gerant attendu".to_string(),
        ));
    }

    limiteur.reussite(&cible);

    depot.executer(|connexion| {
        sessions::fermer(connexion, session.identifiant)?;

        let fermee = sessions::lister(connexion)?
            .into_iter()
            .find(|candidate| candidate.identifiant == session.identifiant);

        fermee.ok_or_else(|| {
            ErreurApplication::Introuvable(format!("session {}", session.identifiant))
        })
    })
}

#[tauri::command]
pub fn lister_sessions(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<Vec<sessions::SessionCaisse>> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(sessions::lister)
}
