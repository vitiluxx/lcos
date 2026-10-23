use std::collections::HashMap;

use rusqlite::Connection;

use crate::erreur::Resultat;

const ECRITURE: &str = "INSERT INTO parametre (cle, valeur) VALUES (?1, ?2)
                        ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur,
                                                       modifie_le = datetime('now')";

const HORODATAGE: &str = "INSERT INTO parametre (cle, valeur) VALUES (?1, datetime('now'))
                          ON CONFLICT(cle) DO UPDATE SET valeur = datetime('now'),
                                                         modifie_le = datetime('now')";

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

pub fn lire_ou(connexion: &Connection, cle: &str, defaut: &str) -> Resultat<String> {
    Ok(lire(connexion, cle)?.unwrap_or_else(|| defaut.to_string()))
}

pub fn lire_entier(connexion: &Connection, cle: &str, defaut: i64) -> Resultat<i64> {
    let valeur = lire(connexion, cle)?
        .and_then(|texte| texte.trim().parse::<i64>().ok())
        .unwrap_or(defaut);

    Ok(valeur)
}

pub fn lire_booleen(connexion: &Connection, cle: &str, defaut: bool) -> Resultat<bool> {
    match lire(connexion, cle)? {
        Some(valeur) => Ok(valeur == "oui"),
        None => Ok(defaut),
    }
}

pub fn ecrire_booleen(connexion: &Connection, cle: &str, valeur: bool) -> Resultat<()> {
    ecrire(connexion, cle, if valeur { "oui" } else { "non" })
}

pub fn ecrire(connexion: &Connection, cle: &str, valeur: &str) -> Resultat<()> {
    connexion.execute(ECRITURE, (cle, valeur))?;

    Ok(())
}

pub fn horodater(connexion: &Connection, cle: &str) -> Resultat<()> {
    connexion.execute(HORODATAGE, (cle,))?;

    Ok(())
}
