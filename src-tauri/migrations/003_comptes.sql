CREATE TABLE IF NOT EXISTS compte (
    identifiant      INTEGER PRIMARY KEY AUTOINCREMENT,
    nom              TEXT    NOT NULL UNIQUE,
    role             TEXT    NOT NULL CHECK (role IN ('gerant', 'caissier')),
    empreinte        TEXT    NOT NULL,
    sel              TEXT    NOT NULL,
    iterations       INTEGER NOT NULL,
    actif            INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    cree_le          TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    modifie_le       TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
);

CREATE UNIQUE INDEX IF NOT EXISTS index_compte_gerant_unique
    ON compte (role) WHERE role = 'gerant';

CREATE INDEX IF NOT EXISTS index_compte_role ON compte (role, actif);

INSERT OR IGNORE INTO parametre (cle, valeur) VALUES
    ('gerant_configure', 'non');
