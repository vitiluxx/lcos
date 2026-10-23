use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::domaine::secret::Empreinte;
use crate::erreur::{ErreurApplication, Resultat};

use super::traduction::conflit_si_contrainte;

pub const LONGUEUR_NOM_MINIMALE: usize = 2;
pub const LONGUEUR_NOM_MAXIMALE: usize = 40;

#[derive(Debug, Serialize)]
pub struct Compte {
    pub identifiant: i64,
    pub nom: String,
    pub role: String,
    pub actif: bool,
    pub cree_le: String,
    pub nombre_ventes: i64,
}

const SELECTION: &str = "SELECT compte.identifiant,
                                compte.nom,
                                compte.role,
                                compte.actif,
                                compte.cree_le,
                                (SELECT COUNT(*) FROM vente WHERE vente.caissier = compte.identifiant)
                           FROM compte";

pub fn valider_nom(nom: &str) -> Resultat<String> {
    let nettoye = nom.trim();

    if nettoye.len() < LONGUEUR_NOM_MINIMALE || nettoye.len() > LONGUEUR_NOM_MAXIMALE {
        return Err(ErreurApplication::Validation(format!(
            "nom : de {LONGUEUR_NOM_MINIMALE} a {LONGUEUR_NOM_MAXIMALE} caracteres"
        )));
    }

    if nettoye.chars().any(|caractere| caractere.is_control()) {
        return Err(ErreurApplication::Validation(
            "nom : caracteres de controle refuses".to_string(),
        ));
    }

    Ok(nettoye.to_string())
}

fn lire_ligne(ligne: &rusqlite::Row) -> rusqlite::Result<Compte> {
    Ok(Compte {
        identifiant: ligne.get(0)?,
        nom: ligne.get(1)?,
        role: ligne.get(2)?,
        actif: ligne.get::<_, i64>(3)? == 1,
        cree_le: ligne.get(4)?,
        nombre_ventes: ligne.get(5)?,
    })
}

pub fn lister(connexion: &Connection, role: &str) -> Resultat<Vec<Compte>> {
    let mut requete =
        connexion.prepare(&format!("{SELECTION} WHERE compte.role = ?1 ORDER BY compte.nom"))?;
    let mut lignes = requete.query([role])?;
    let mut comptes = Vec::new();

    while let Some(ligne) = lignes.next()? {
        comptes.push(lire_ligne(ligne)?);
    }

    Ok(comptes)
}

pub fn lire(connexion: &Connection, identifiant: i64) -> Resultat<Option<Compte>> {
    let compte = connexion
        .query_row(
            &format!("{SELECTION} WHERE compte.identifiant = ?1"),
            [identifiant],
            lire_ligne,
        )
        .optional()?;

    Ok(compte)
}

pub fn lire_par_role(connexion: &Connection, role: &str) -> Resultat<Option<Compte>> {
    let compte = connexion
        .query_row(
            &format!("{SELECTION} WHERE compte.role = ?1"),
            [role],
            lire_ligne,
        )
        .optional()?;

    Ok(compte)
}

pub fn exiger(connexion: &Connection, identifiant: i64) -> Resultat<Compte> {
    lire(connexion, identifiant)?
        .ok_or_else(|| ErreurApplication::Introuvable(format!("compte {identifiant}")))
}

pub fn lire_empreinte(connexion: &Connection, identifiant: i64) -> Resultat<Option<Empreinte>> {
    let empreinte = connexion
        .query_row(
            "SELECT empreinte, sel, iterations FROM compte WHERE identifiant = ?1",
            [identifiant],
            |ligne| {
                Ok(Empreinte {
                    empreinte: ligne.get(0)?,
                    sel: ligne.get(1)?,
                    iterations: ligne.get::<_, i64>(2)? as u32,
                })
            },
        )
        .optional()?;

    Ok(empreinte)
}

pub fn creer(
    connexion: &Connection,
    nom: &str,
    role: &str,
    empreinte: &Empreinte,
) -> Resultat<i64> {
    connexion
        .execute(
            "INSERT INTO compte (nom, role, empreinte, sel, iterations)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                nom,
                role,
                &empreinte.empreinte,
                &empreinte.sel,
                empreinte.iterations as i64,
            ),
        )
        .map_err(|erreur| {
            conflit_si_contrainte(erreur, &format!("le nom {nom} est deja pris"))
        })?;

    Ok(connexion.last_insert_rowid())
}

pub fn renommer(connexion: &Connection, identifiant: i64, nom: &str) -> Resultat<()> {
    let touchees = connexion
        .execute(
            "UPDATE compte SET nom = ?2, modifie_le = datetime('now', 'localtime')
              WHERE identifiant = ?1",
            (identifiant, nom),
        )
        .map_err(|erreur| {
            conflit_si_contrainte(erreur, &format!("le nom {nom} est deja pris"))
        })?;

    exiger_touchee(touchees, identifiant)
}

pub fn definir_actif(connexion: &Connection, identifiant: i64, actif: bool) -> Resultat<()> {
    let touchees = connexion.execute(
        "UPDATE compte SET actif = ?2, modifie_le = datetime('now', 'localtime')
          WHERE identifiant = ?1",
        (identifiant, i64::from(actif)),
    )?;

    exiger_touchee(touchees, identifiant)
}

pub fn definir_secret(
    connexion: &Connection,
    identifiant: i64,
    empreinte: &Empreinte,
) -> Resultat<()> {
    let touchees = connexion.execute(
        "UPDATE compte SET empreinte = ?2, sel = ?3, iterations = ?4,
                           modifie_le = datetime('now', 'localtime')
          WHERE identifiant = ?1",
        (
            identifiant,
            &empreinte.empreinte,
            &empreinte.sel,
            empreinte.iterations as i64,
        ),
    )?;

    exiger_touchee(touchees, identifiant)
}

pub fn supprimer(connexion: &Connection, identifiant: i64) -> Resultat<()> {
    let touchees = connexion
        .execute("DELETE FROM compte WHERE identifiant = ?1", [identifiant])
        .map_err(|erreur| {
            conflit_si_contrainte(
                erreur,
                "ce compte a des ventes ou des sessions : desactive-le au lieu de le supprimer",
            )
        })?;

    exiger_touchee(touchees, identifiant)
}

fn exiger_touchee(touchees: usize, identifiant: i64) -> Resultat<()> {
    if touchees == 0 {
        return Err(ErreurApplication::Introuvable(format!(
            "compte {identifiant}"
        )));
    }

    Ok(())
}
