/*
 * Compteurs persistants de la caisse. Une remise a zero efface les ventes
 * mais jamais ces valeurs : grands totaux, numero de ticket et nombre de
 * remises a zero traversent toutes les clotures.
 */
use std::collections::HashMap;

use rusqlite::Connection;

use crate::erreur::Resultat;

pub const NUMERO_TICKET: &str = "numero_ticket";
pub const REMISE_A_ZERO: &str = "remise_a_zero";
pub const GRAND_TOTAL_CENTIMES: &str = "grand_total_centimes";
pub const GRAND_TOTAL_ARTICLES: &str = "grand_total_articles";
pub const GRAND_TOTAL_VENTES: &str = "grand_total_ventes";

const LECTURE: &str = "SELECT valeur FROM compteur WHERE cle = ?1";

const INCREMENT: &str = "INSERT INTO compteur (cle, valeur) VALUES (?1, ?2)
                         ON CONFLICT(cle) DO UPDATE SET valeur = valeur + excluded.valeur";

pub fn lire(connexion: &Connection, cle: &str) -> Resultat<i64> {
    let mut requete = connexion.prepare(LECTURE)?;
    let mut lignes = requete.query([cle])?;

    match lignes.next()? {
        Some(ligne) => Ok(ligne.get(0)?),
        None => Ok(0),
    }
}

pub fn lire_tous(connexion: &Connection) -> Resultat<HashMap<String, i64>> {
    let mut requete = connexion.prepare("SELECT cle, valeur FROM compteur")?;
    let mut lignes = requete.query([])?;
    let mut valeurs = HashMap::new();

    while let Some(ligne) = lignes.next()? {
        valeurs.insert(ligne.get(0)?, ligne.get(1)?);
    }

    Ok(valeurs)
}

pub fn incrementer(connexion: &Connection, cle: &str, pas: i64) -> Resultat<i64> {
    connexion.execute(INCREMENT, (cle, pas))?;

    lire(connexion, cle)
}
