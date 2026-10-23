use rusqlite::Connection;

use crate::erreur::Resultat;

/// Heure locale du poste, lue par SQLite : une seule source de temps pour
/// l horodatage des ventes et pour l affichage.
pub fn maintenant(connexion: &Connection) -> Resultat<String> {
    let valeur =
        connexion.query_row("SELECT datetime('now', 'localtime')", [], |ligne| ligne.get(0))?;

    Ok(valeur)
}
