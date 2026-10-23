/*
 * Archive des rapports Z. Le texte imprime est conserve tel quel : apres
 * effacement des ventes, c est la seule trace lisible de la periode.
 */
use rusqlite::Connection;
use serde::Serialize;

use crate::domaine::horodatage;
use crate::domaine::ticket::DonneesRapportZ;
use crate::erreur::Resultat;

pub const LIMITE_HISTORIQUE: i64 = 100;

#[derive(Debug, Serialize)]
pub struct RapportArchive {
    pub identifiant: i64,
    pub numero: i64,
    pub genere_le: String,
    pub debut_le: String,
    pub fin_le: String,
    pub nombre_ventes: i64,
    pub nombre_articles: i64,
    pub total_centimes: i64,
    pub premier_ticket: i64,
    pub dernier_ticket: i64,
    pub texte: String,
}

const SELECTION: &str = "SELECT identifiant, numero, genere_le, debut_le, fin_le,
                                nombre_ventes, nombre_articles, total_centimes,
                                premier_ticket, dernier_ticket, texte
                           FROM rapport_z";

fn lire_ligne(ligne: &rusqlite::Row) -> rusqlite::Result<RapportArchive> {
    let genere: String = ligne.get(2)?;

    Ok(RapportArchive {
        identifiant: ligne.get(0)?,
        numero: ligne.get(1)?,
        genere_le: horodatage::lisible(&genere),
        debut_le: ligne.get::<_, Option<String>>(3)?.unwrap_or_default(),
        fin_le: ligne.get::<_, Option<String>>(4)?.unwrap_or_default(),
        nombre_ventes: ligne.get(5)?,
        nombre_articles: ligne.get(6)?,
        total_centimes: ligne.get(7)?,
        premier_ticket: ligne.get::<_, Option<i64>>(8)?.unwrap_or_default(),
        dernier_ticket: ligne.get::<_, Option<i64>>(9)?.unwrap_or_default(),
        texte: ligne.get(10)?,
    })
}

pub fn archiver(
    connexion: &Connection,
    donnees: &DonneesRapportZ,
    texte: &str,
) -> Resultat<i64> {
    connexion.execute(
        "INSERT INTO rapport_z (numero, debut_le, fin_le, nombre_ventes, nombre_articles,
                                total_centimes, premier_ticket, dernier_ticket, texte)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        (
            donnees.numero,
            &donnees.debut_le,
            &donnees.fin_le,
            donnees.nombre_ventes,
            donnees.nombre_articles,
            donnees.total_centimes,
            donnees.premier_ticket,
            donnees.dernier_ticket,
            texte,
        ),
    )?;

    Ok(connexion.last_insert_rowid())
}

pub fn lister(connexion: &Connection) -> Resultat<Vec<RapportArchive>> {
    let mut requete = connexion.prepare(&format!(
        "{SELECTION} ORDER BY numero DESC LIMIT {LIMITE_HISTORIQUE}"
    ))?;
    let mut lignes = requete.query([])?;
    let mut rapports = Vec::new();

    while let Some(ligne) = lignes.next()? {
        rapports.push(lire_ligne(ligne)?);
    }

    Ok(rapports)
}
