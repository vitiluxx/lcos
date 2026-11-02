use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use crate::erreur::{ErreurApplication, Resultat};

/// Le repli du journal ferme la marche. En mode WAL, SQLite recopie le
/// journal dans la base des qu il depasse mille pages, soit quatre mega-octets
/// : au-dela, chaque ecriture retente cette recopie, et la fenetre se fige une
/// seconde ou deux le temps qu elle se fasse. Au lancement, aucun autre
/// lecteur ne tient la base, donc le repli aboutit et repart de zero.
const REGLAGES: &str = "PRAGMA journal_mode = WAL; \
                        PRAGMA foreign_keys = ON; \
                        PRAGMA synchronous = NORMAL; \
                        PRAGMA busy_timeout = 5000; \
                        PRAGMA wal_checkpoint(TRUNCATE);";

pub struct Depot {
    connexion: Mutex<Connection>,
    chemin: PathBuf,
}

impl Depot {
    pub fn ouvrir(chemin: &Path) -> Resultat<Self> {
        if let Some(dossier) = chemin.parent() {
            std::fs::create_dir_all(dossier)?;
        }

        let connexion = Connection::open(chemin)?;
        connexion.execute_batch(REGLAGES)?;

        Ok(Self {
            connexion: Mutex::new(connexion),
            chemin: chemin.to_path_buf(),
        })
    }

    pub fn executer<T, A>(&self, action: A) -> Resultat<T>
    where
        A: FnOnce(&Connection) -> Resultat<T>,
    {
        let connexion = self
            .connexion
            .lock()
            .map_err(|_| ErreurApplication::VerrouCorrompu)?;

        action(&connexion)
    }

    pub fn chemin(&self) -> &Path {
        &self.chemin
    }
}
