use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

use crate::erreur::{ErreurApplication, Resultat};

const REGLAGES: &str = "PRAGMA journal_mode = WAL; \
                        PRAGMA foreign_keys = ON; \
                        PRAGMA synchronous = NORMAL; \
                        PRAGMA busy_timeout = 5000;";

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
