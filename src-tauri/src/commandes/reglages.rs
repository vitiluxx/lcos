use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

use crate::depot::{parametres, Depot};
use crate::domaine::ticket;
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::RegistreJetons;

const CLE_NOM_COMMERCE: &str = "nom_commerce";
const CLE_DEVISE: &str = "symbole_devise";
const CLE_TICKET_ACTIF: &str = "ticket_actif";
const CLE_IMPRESSION_AUTO: &str = "ticket_impression_auto";
const CLE_COLONNES: &str = "ticket_largeur_colonnes";
const CLE_ENTETE: &str = "ticket_entete";
const CLE_PIED: &str = "ticket_pied";

const LONGUEUR_NOM_MAXIMALE: usize = 40;
const LONGUEUR_DEVISE_MAXIMALE: usize = 8;
const LONGUEUR_MENTION_MAXIMALE: usize = 60;

#[derive(Debug, Serialize)]
pub struct Reglages {
    pub nom_commerce: String,
    pub devise: String,
    pub ticket_actif: bool,
    pub ticket_impression_auto: bool,
    pub ticket_largeur_colonnes: i64,
    pub ticket_entete: String,
    pub ticket_pied: String,
}

/// Lecture partagee : la caisse, le ticket et le rapport Z s appuient dessus.
pub fn charger(connexion: &Connection) -> Resultat<Reglages> {
    Ok(Reglages {
        nom_commerce: parametres::lire_ou(connexion, CLE_NOM_COMMERCE, "Mon commerce")?,
        devise: parametres::lire_ou(connexion, CLE_DEVISE, "EUR")?,
        ticket_actif: parametres::lire_booleen(connexion, CLE_TICKET_ACTIF, true)?,
        ticket_impression_auto: parametres::lire_booleen(connexion, CLE_IMPRESSION_AUTO, false)?,
        ticket_largeur_colonnes: parametres::lire_entier(
            connexion,
            CLE_COLONNES,
            ticket::COLONNES_58MM as i64,
        )?,
        ticket_entete: parametres::lire_ou(connexion, CLE_ENTETE, "")?,
        ticket_pied: parametres::lire_ou(connexion, CLE_PIED, "")?,
    })
}

#[tauri::command]
pub fn recuperer_reglages(depot: State<Depot>) -> Resultat<Reglages> {
    depot.executer(charger)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn enregistrer_reglages(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    nom_commerce: String,
    devise: String,
    ticket_actif: bool,
    ticket_impression_auto: bool,
    ticket_largeur_colonnes: i64,
    ticket_entete: String,
    ticket_pied: String,
) -> Resultat<Reglages> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let nom = valider_texte(&nom_commerce, 2, LONGUEUR_NOM_MAXIMALE, "nom du commerce")?;
    let symbole = valider_texte(&devise, 1, LONGUEUR_DEVISE_MAXIMALE, "devise")?;
    let entete = valider_mention(&ticket_entete, "entete du ticket")?;
    let pied = valider_mention(&ticket_pied, "pied du ticket")?;
    let colonnes = ticket::borner_colonnes(ticket_largeur_colonnes) as i64;

    depot.executer(|connexion| {
        parametres::ecrire(connexion, CLE_NOM_COMMERCE, &nom)?;
        parametres::ecrire(connexion, CLE_DEVISE, &symbole)?;
        parametres::ecrire_booleen(connexion, CLE_TICKET_ACTIF, ticket_actif)?;
        parametres::ecrire_booleen(connexion, CLE_IMPRESSION_AUTO, ticket_impression_auto)?;
        parametres::ecrire(connexion, CLE_COLONNES, &colonnes.to_string())?;
        parametres::ecrire(connexion, CLE_ENTETE, &entete)?;
        parametres::ecrire(connexion, CLE_PIED, &pied)?;

        charger(connexion)
    })
}

/// Activation ou desactivation du ticket de caisse, usage quotidien.
#[tauri::command]
pub fn basculer_ticket(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    actif: bool,
) -> Resultat<Reglages> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(|connexion| {
        parametres::ecrire_booleen(connexion, CLE_TICKET_ACTIF, actif)?;

        charger(connexion)
    })
}

fn valider_texte(
    valeur: &str,
    minimum: usize,
    maximum: usize,
    libelle: &str,
) -> Resultat<String> {
    let nettoye = valeur.trim();

    if nettoye.len() < minimum || nettoye.len() > maximum {
        return Err(ErreurApplication::Validation(format!(
            "{libelle} : de {minimum} a {maximum} caracteres"
        )));
    }

    if nettoye.chars().any(|caractere| caractere.is_control()) {
        return Err(ErreurApplication::Validation(format!(
            "{libelle} : caracteres de controle refuses"
        )));
    }

    Ok(nettoye.to_string())
}

fn valider_mention(valeur: &str, libelle: &str) -> Resultat<String> {
    let nettoye = valeur.trim();

    if nettoye.is_empty() {
        return Ok(String::new());
    }

    valider_texte(nettoye, 1, LONGUEUR_MENTION_MAXIMALE, libelle)
}
