/*
 * Sessions de caisse. Un seul poste, donc une seule session ouverte a la fois :
 * la contrainte est tenue par un index unique partiel, pas par le front.
 */
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::erreur::{ErreurApplication, Resultat};

use super::traduction::conflit_si_contrainte;

pub const ETAT_OUVERTE: &str = "ouverte";
pub const ETAT_FERMEE: &str = "fermee";
pub const LIMITE_HISTORIQUE: i64 = 50;

#[derive(Debug, Serialize)]
pub struct SessionCaisse {
    pub identifiant: i64,
    pub caissier: i64,
    pub caissier_nom: String,
    pub ouverte_le: String,
    pub fermee_le: Option<String>,
    pub fond_centimes: i64,
    pub etat: String,
    pub nombre_ventes: i64,
    pub nombre_articles: i64,
    pub total_centimes: i64,
}

const SELECTION: &str = "SELECT session_caisse.identifiant,
                                session_caisse.caissier,
                                compte.nom,
                                session_caisse.ouverte_le,
                                session_caisse.fermee_le,
                                session_caisse.fond_centimes,
                                session_caisse.etat,
                                COALESCE((SELECT COUNT(*) FROM vente
                                           WHERE vente.session = session_caisse.identifiant), 0),
                                COALESCE((SELECT SUM(nombre_articles) FROM vente
                                           WHERE vente.session = session_caisse.identifiant), 0),
                                COALESCE((SELECT SUM(total_centimes) FROM vente
                                           WHERE vente.session = session_caisse.identifiant), 0)
                           FROM session_caisse
                           JOIN compte ON compte.identifiant = session_caisse.caissier";

fn lire_ligne(ligne: &rusqlite::Row) -> rusqlite::Result<SessionCaisse> {
    Ok(SessionCaisse {
        identifiant: ligne.get(0)?,
        caissier: ligne.get(1)?,
        caissier_nom: ligne.get(2)?,
        ouverte_le: ligne.get(3)?,
        fermee_le: ligne.get(4)?,
        fond_centimes: ligne.get(5)?,
        etat: ligne.get(6)?,
        nombre_ventes: ligne.get(7)?,
        nombre_articles: ligne.get(8)?,
        total_centimes: ligne.get(9)?,
    })
}

pub fn ouverte(connexion: &Connection) -> Resultat<Option<SessionCaisse>> {
    let session = connexion
        .query_row(
            &format!("{SELECTION} WHERE session_caisse.etat = ?1"),
            [ETAT_OUVERTE],
            lire_ligne,
        )
        .optional()?;

    Ok(session)
}

pub fn exiger_ouverte(connexion: &Connection) -> Resultat<SessionCaisse> {
    ouverte(connexion)?.ok_or_else(|| {
        ErreurApplication::Conflit("aucune session de caisse ouverte".to_string())
    })
}

pub fn lister(connexion: &Connection) -> Resultat<Vec<SessionCaisse>> {
    let mut requete = connexion.prepare(&format!(
        "{SELECTION} ORDER BY session_caisse.identifiant DESC LIMIT {LIMITE_HISTORIQUE}"
    ))?;
    let mut lignes = requete.query([])?;
    let mut sessions = Vec::new();

    while let Some(ligne) = lignes.next()? {
        sessions.push(lire_ligne(ligne)?);
    }

    Ok(sessions)
}

pub fn ouvrir(connexion: &Connection, caissier: i64, fond_centimes: i64) -> Resultat<i64> {
    connexion
        .execute(
            "INSERT INTO session_caisse (caissier, fond_centimes) VALUES (?1, ?2)",
            (caissier, fond_centimes),
        )
        .map_err(|erreur| {
            conflit_si_contrainte(
                erreur,
                "une session est deja ouverte : ferme-la avant d en ouvrir une autre",
            )
        })?;

    Ok(connexion.last_insert_rowid())
}

pub fn fermer(connexion: &Connection, identifiant: i64) -> Resultat<()> {
    let touchees = connexion.execute(
        "UPDATE session_caisse
            SET etat = ?2, fermee_le = datetime('now', 'localtime')
          WHERE identifiant = ?1 AND etat = ?3",
        (identifiant, ETAT_FERMEE, ETAT_OUVERTE),
    )?;

    if touchees == 0 {
        return Err(ErreurApplication::Conflit(format!(
            "session {identifiant} deja fermee"
        )));
    }

    Ok(())
}

/// Supprime les sessions fermees et sans vente : utilise par la remise a zero.
pub fn purger_fermees(connexion: &Connection) -> Resultat<usize> {
    let supprimees = connexion.execute(
        "DELETE FROM session_caisse
          WHERE etat = ?1
            AND NOT EXISTS (SELECT 1 FROM vente WHERE vente.session = session_caisse.identifiant)",
        [ETAT_FERMEE],
    )?;

    Ok(supprimees)
}
