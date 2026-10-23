CREATE TABLE IF NOT EXISTS rayon (
    identifiant INTEGER PRIMARY KEY AUTOINCREMENT,
    libelle     TEXT    NOT NULL UNIQUE,
    ordre       INTEGER NOT NULL DEFAULT 0,
    actif       INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    cree_le     TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    modifie_le  TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS produit (
    identifiant   INTEGER PRIMARY KEY AUTOINCREMENT,
    rayon         INTEGER NOT NULL REFERENCES rayon (identifiant),
    libelle       TEXT    NOT NULL,
    code_barres   TEXT,
    prix_centimes INTEGER NOT NULL CHECK (prix_centimes >= 0),
    stock         INTEGER NOT NULL DEFAULT 0,
    suivi_stock   INTEGER NOT NULL DEFAULT 1 CHECK (suivi_stock IN (0, 1)),
    actif         INTEGER NOT NULL DEFAULT 1 CHECK (actif IN (0, 1)),
    cree_le       TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    modifie_le    TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
);

CREATE UNIQUE INDEX IF NOT EXISTS index_produit_code_barres
    ON produit (code_barres) WHERE code_barres IS NOT NULL;

CREATE INDEX IF NOT EXISTS index_produit_rayon ON produit (rayon, actif);
CREATE INDEX IF NOT EXISTS index_produit_libelle ON produit (libelle);
