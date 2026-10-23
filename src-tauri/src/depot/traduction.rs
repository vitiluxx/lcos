use crate::erreur::ErreurApplication;

/// Une contrainte SQLite violee est une erreur metier, pas une panne : elle
/// doit remonter au caissier avec un message comprehensible.
pub fn conflit_si_contrainte(erreur: rusqlite::Error, message: &str) -> ErreurApplication {
    match &erreur {
        rusqlite::Error::SqliteFailure(interne, _)
            if interne.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            ErreurApplication::Conflit(message.to_string())
        }
        _ => ErreurApplication::BaseDeDonnees(erreur),
    }
}
