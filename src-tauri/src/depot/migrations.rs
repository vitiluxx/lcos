use rusqlite::Connection;

use crate::erreur::Resultat;

struct Migration {
    version: i64,
    nom: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        nom: "socle",
        sql: include_str!("../../migrations/001_socle.sql"),
    },
    Migration {
        version: 2,
        nom: "peripheriques",
        sql: include_str!("../../migrations/002_peripheriques.sql"),
    },
    Migration {
        version: 3,
        nom: "comptes",
        sql: include_str!("../../migrations/003_comptes.sql"),
    },
    Migration {
        version: 4,
        nom: "catalogue",
        sql: include_str!("../../migrations/004_catalogue.sql"),
    },
    Migration {
        version: 5,
        nom: "ventes",
        sql: include_str!("../../migrations/005_ventes.sql"),
    },
    Migration {
        version: 6,
        nom: "annulation",
        sql: include_str!("../../migrations/006_annulation.sql"),
    },
];

const CREATION_JOURNAL: &str = "CREATE TABLE IF NOT EXISTS migration (
    version      INTEGER PRIMARY KEY,
    nom          TEXT    NOT NULL,
    appliquee_le TEXT    NOT NULL DEFAULT (datetime('now'))
);";

pub fn appliquer(connexion: &Connection) -> Resultat<usize> {
    connexion.execute_batch(CREATION_JOURNAL)?;

    let mut posees = 0usize;

    for migration in MIGRATIONS {
        if deja_appliquee(connexion, migration.version)? {
            continue;
        }

        connexion.execute_batch("BEGIN;")?;
        connexion.execute_batch(migration.sql)?;
        connexion.execute(
            "INSERT INTO migration (version, nom) VALUES (?1, ?2)",
            (migration.version, migration.nom),
        )?;
        connexion.execute_batch("COMMIT;")?;

        posees += 1;
    }

    Ok(posees)
}

pub fn nombre_appliquees(connexion: &Connection) -> Resultat<i64> {
    let total = connexion.query_row("SELECT COUNT(*) FROM migration", [], |ligne| ligne.get(0))?;

    Ok(total)
}

fn deja_appliquee(connexion: &Connection, version: i64) -> Resultat<bool> {
    let presence: i64 = connexion.query_row(
        "SELECT COUNT(*) FROM migration WHERE version = ?1",
        [version],
        |ligne| ligne.get(0),
    )?;

    Ok(presence > 0)
}
