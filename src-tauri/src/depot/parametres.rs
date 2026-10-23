use std::collections::HashMap;

use rusqlite::Connection;

use crate::erreur::Resultat;

const ECRITURE: &str = "INSERT INTO parametre (cle, valeur) VALUES (?1, ?2)
                        ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur,
                                                       modifie_le = datetime('now')";

const HORODATAGE: &str = "INSERT INTO parametre (cle, valeur) VALUES (?1, datetime('now'))
                          ON CONFLICT(cle) DO UPDATE SET valeur = datetime('now'),
                                                         modifie_le = datetime('now')";

#[allow(dead_code)] // utilise par le module catalogue
pub fn lire(connexion: &Connection, cle: &str) -> Resultat<Option<String>> {
    let mut requete = connexion.prepare("SELECT valeur FROM parametre WHERE cle = ?1")?;
    let mut lignes = requete.query([cle])?;

    match lignes.next()? {
        Some(ligne) => Ok(Some(ligne.get(0)?)),
        None => Ok(None),
    }
}

pub fn lire_prefixe(connexion: &Connection, prefixe: &str) -> Resultat<HashMap<String, String>> {
    let mut requete = connexion.prepare("SELECT cle, valeur FROM parametre WHERE cle LIKE ?1")?;
    let mut lignes = requete.query([format!("{prefixe}%")])?;
    let mut valeurs = HashMap::new();

    while let Some(ligne) = lignes.next()? {
        valeurs.insert(ligne.get(0)?, ligne.get(1)?);
    }

    Ok(valeurs)
}

pub fn ecrire(connexion: &Connection, cle: &str, valeur: &str) -> Resultat<()> {
    connexion.execute(ECRITURE, (cle, valeur))?;

    Ok(())
}

pub fn horodater(connexion: &Connection, cle: &str) -> Resultat<()> {
    connexion.execute(HORODATAGE, (cle,))?;

    Ok(())
}
