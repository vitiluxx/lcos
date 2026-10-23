use serde::Serialize;
use tauri::State;

use crate::depot::{comptes, parametres, Depot};
use crate::domaine::secret;
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::{Limiteur, RegistreJetons};

const CLE_GERANT_CONFIGURE: &str = "gerant_configure";
const CIBLE_GERANT: &str = "gerant";

#[derive(Serialize)]
pub struct AccesGerant {
    pub jeton: String,
    pub nom: String,
}

#[derive(Serialize)]
pub struct EtatProtection {
    pub gerant_configure: bool,
    pub nom_gerant: String,
}

#[tauri::command]
pub fn recuperer_etat_protection(depot: State<Depot>) -> Resultat<EtatProtection> {
    depot.executer(|connexion| {
        let gerant = comptes::lire_par_role(connexion, ROLE_GERANT)?;

        Ok(EtatProtection {
            gerant_configure: gerant.is_some(),
            nom_gerant: gerant.map(|compte| compte.nom).unwrap_or_default(),
        })
    })
}

/// Premier demarrage : creation du compte gerant. Refusee des qu il existe.
#[tauri::command]
pub fn initialiser_gerant(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    nom: String,
    code: String,
) -> Resultat<AccesGerant> {
    let nom_valide = comptes::valider_nom(&nom)?;
    let empreinte = secret::sceller(code.trim())?;

    let identifiant = depot.executer(|connexion| {
        if comptes::lire_par_role(connexion, ROLE_GERANT)?.is_some() {
            return Err(ErreurApplication::Conflit(
                "le compte gerant existe deja".to_string(),
            ));
        }

        let identifiant = comptes::creer(connexion, &nom_valide, ROLE_GERANT, &empreinte)?;
        parametres::ecrire_booleen(connexion, CLE_GERANT_CONFIGURE, true)?;

        Ok(identifiant)
    })?;

    Ok(AccesGerant {
        jeton: jetons.emettre(identifiant, ROLE_GERANT)?,
        nom: nom_valide,
    })
}

/// Ouvre l acces au menu parametrage contre le code secret du gerant.
#[tauri::command]
pub fn ouvrir_session_gerant(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    limiteur: State<Limiteur>,
    code: String,
) -> Resultat<AccesGerant> {
    limiteur.verifier(CIBLE_GERANT)?;

    let gerant = depot
        .executer(|connexion| comptes::lire_par_role(connexion, ROLE_GERANT))?
        .ok_or_else(|| {
            ErreurApplication::Conflit("aucun gerant : configurer la caisse d abord".to_string())
        })?;

    let empreinte = depot
        .executer(|connexion| comptes::lire_empreinte(connexion, gerant.identifiant))?
        .ok_or_else(|| ErreurApplication::Introuvable("empreinte du gerant".to_string()))?;

    if !secret::verifier(code.trim(), &empreinte) {
        limiteur.echec(CIBLE_GERANT);

        return Err(ErreurApplication::AccesRefuse("code incorrect".to_string()));
    }

    limiteur.reussite(CIBLE_GERANT);

    Ok(AccesGerant {
        jeton: jetons.emettre(gerant.identifiant, ROLE_GERANT)?,
        nom: gerant.nom,
    })
}

#[tauri::command]
pub fn fermer_session_gerant(jetons: State<RegistreJetons>, jeton: String) -> Resultat<()> {
    jetons.revoquer(&jeton);

    Ok(())
}

#[tauri::command]
pub fn modifier_secret_gerant(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    limiteur: State<Limiteur>,
    jeton: String,
    ancien_code: String,
    nouveau_code: String,
) -> Resultat<()> {
    let acces = jetons.exiger(&jeton, ROLE_GERANT)?;

    limiteur.verifier(CIBLE_GERANT)?;

    let empreinte_actuelle = depot
        .executer(|connexion| comptes::lire_empreinte(connexion, acces.compte))?
        .ok_or_else(|| ErreurApplication::Introuvable("empreinte du gerant".to_string()))?;

    if !secret::verifier(ancien_code.trim(), &empreinte_actuelle) {
        limiteur.echec(CIBLE_GERANT);

        return Err(ErreurApplication::AccesRefuse(
            "ancien code incorrect".to_string(),
        ));
    }

    limiteur.reussite(CIBLE_GERANT);

    let nouvelle = secret::sceller(nouveau_code.trim())?;

    depot.executer(|connexion| comptes::definir_secret(connexion, acces.compte, &nouvelle))
}

#[tauri::command]
pub fn renommer_gerant(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    nom: String,
) -> Resultat<()> {
    let acces = jetons.exiger(&jeton, ROLE_GERANT)?;
    let nom_valide = comptes::valider_nom(&nom)?;

    depot.executer(|connexion| comptes::renommer(connexion, acces.compte, &nom_valide))
}
