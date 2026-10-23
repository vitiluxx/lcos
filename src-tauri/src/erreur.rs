use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum ErreurApplication {
    #[error("erreur base de donnees: {0}")]
    BaseDeDonnees(#[from] rusqlite::Error),

    #[error("erreur systeme de fichiers: {0}")]
    SystemeFichiers(#[from] std::io::Error),

    #[error("verrou de connexion corrompu")]
    VerrouCorrompu,

    #[error("donnee invalide: {0}")]
    Validation(String),

    #[error("acces refuse: {0}")]
    AccesRefuse(String),

    #[error("introuvable: {0}")]
    Introuvable(String),

    #[error("operation refusee: {0}")]
    Conflit(String),
}

impl Serialize for ErreurApplication {
    fn serialize<S: Serializer>(&self, serialiseur: S) -> Result<S::Ok, S::Error> {
        serialiseur.serialize_str(&self.to_string())
    }
}

pub type Resultat<T> = Result<T, ErreurApplication>;
