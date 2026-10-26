use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

use crate::depot::{parametres, Depot};
use crate::domaine::{monnaie, ticket};
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::RegistreJetons;

const CLE_NOM_COMMERCE: &str = "nom_commerce";
const CLE_DEVISE: &str = "symbole_devise";
const CLE_TICKET_ACTIF: &str = "ticket_actif";
const CLE_IMPRESSION_RECU: &str = "impression_recu";
const CLE_DECIMALES: &str = "monnaie_decimales";
const CLE_ANNULATION_VENTE: &str = "annulation_vente";
const CLE_COLONNES: &str = "ticket_largeur_colonnes";
const CLE_ENTETE: &str = "ticket_entete";
const CLE_PIED: &str = "ticket_pied";

const LONGUEUR_NOM_MAXIMALE: usize = 40;
const LONGUEUR_DEVISE_MAXIMALE: usize = 8;
const LONGUEUR_MENTION_MAXIMALE: usize = 60;

/// Devises sans subdivision : aucune valeur apres la virgule n a de sens.
/// Sert de valeur par defaut tant que le gerant n a pas tranche lui-meme.
const DEVISES_SANS_DECIMALE: &[&str] = &["FCFA", "CFA", "F CFA", "XOF", "XAF", "KMF", "GNF"];

#[derive(Debug, Serialize)]
pub struct Reglages {
    pub nom_commerce: String,
    pub devise: String,
    pub ticket_actif: bool,
    /// Impression du recu papier apres chaque vente. Desactivee, la caisse se
    /// contente d un eclair de confirmation a l ecran. Le rapport Z n est
    /// jamais concerne : il s imprime toujours.
    pub impression_recu: bool,
    /// 2 pour l euro, 0 pour une devise sans subdivision comme le FCFA.
    pub decimales: i64,
    /// Annulation d une vente deja encaissee depuis l ecran Session. Fermee
    /// par defaut : c est le gerant qui ouvre la porte, et une annulation
    /// laisse une trace nominative.
    pub annulation_vente: bool,
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
        impression_recu: parametres::lire_booleen(connexion, CLE_IMPRESSION_RECU, true)?,
        decimales: charger_decimales(connexion)?,
        annulation_vente: parametres::lire_booleen(connexion, CLE_ANNULATION_VENTE, false)?,
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
    decimales: i64,
    ticket_actif: bool,
    impression_recu: bool,
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
    let decimales_retenues = monnaie::borner_decimales(decimales);

    depot.executer(|connexion| {
        parametres::ecrire(connexion, CLE_NOM_COMMERCE, &nom)?;
        parametres::ecrire(connexion, CLE_DEVISE, &symbole)?;
        parametres::ecrire(connexion, CLE_DECIMALES, &decimales_retenues.to_string())?;
        parametres::ecrire_booleen(connexion, CLE_TICKET_ACTIF, ticket_actif)?;
        parametres::ecrire_booleen(connexion, CLE_IMPRESSION_RECU, impression_recu)?;
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

/// Bascule rapide de l impression des recus, usage quotidien du gerant.
#[tauri::command]
pub fn basculer_impression_recu(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    actif: bool,
) -> Resultat<Reglages> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(|connexion| {
        parametres::ecrire_booleen(connexion, CLE_IMPRESSION_RECU, actif)?;

        charger(connexion)
    })
}

/// Ouverture ou fermeture de l annulation des ventes deja encaissees.
#[tauri::command]
pub fn basculer_annulation_vente(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    actif: bool,
) -> Resultat<Reglages> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(|connexion| {
        parametres::ecrire_booleen(connexion, CLE_ANNULATION_VENTE, actif)?;

        charger(connexion)
    })
}

/// Tant que le gerant n a rien choisi, la devise decide : une caisse reglee en
/// FCFA demarre en entiers pleins, une caisse en euro garde ses centimes.
fn charger_decimales(connexion: &Connection) -> Resultat<i64> {
    let devise = parametres::lire_ou(connexion, CLE_DEVISE, "EUR")?;

    match parametres::lire(connexion, CLE_DECIMALES)? {
        Some(valeur) => Ok(monnaie::borner_decimales(
            valeur.trim().parse::<i64>().unwrap_or(monnaie::DECIMALES_DEFAUT),
        )),
        None => Ok(decimales_par_devise(&devise)),
    }
}

pub fn decimales_par_devise(devise: &str) -> i64 {
    let nettoyee = devise.trim().to_uppercase();

    if DEVISES_SANS_DECIMALE
        .iter()
        .any(|connue| *connue == nettoyee)
    {
        return 0;
    }

    monnaie::DECIMALES_DEFAUT
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnait_les_devises_sans_decimale() {
        assert_eq!(decimales_par_devise("FCFA"), 0);
        assert_eq!(decimales_par_devise(" fcfa "), 0);
        assert_eq!(decimales_par_devise("XOF"), 0);
        assert_eq!(decimales_par_devise("EUR"), monnaie::DECIMALES_DEFAUT);
        assert_eq!(decimales_par_devise("USD"), monnaie::DECIMALES_DEFAUT);
    }
}
