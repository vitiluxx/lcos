CREATE TABLE IF NOT EXISTS session_caisse (
    identifiant   INTEGER PRIMARY KEY AUTOINCREMENT,
    caissier      INTEGER NOT NULL REFERENCES compte (identifiant),
    ouverte_le    TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    fermee_le     TEXT,
    fond_centimes INTEGER NOT NULL DEFAULT 0 CHECK (fond_centimes >= 0),
    etat          TEXT    NOT NULL DEFAULT 'ouverte' CHECK (etat IN ('ouverte', 'fermee'))
);

CREATE UNIQUE INDEX IF NOT EXISTS index_session_une_seule_ouverte
    ON session_caisse (etat) WHERE etat = 'ouverte';

CREATE INDEX IF NOT EXISTS index_session_caissier ON session_caisse (caissier);

CREATE TABLE IF NOT EXISTS vente (
    identifiant     INTEGER PRIMARY KEY AUTOINCREMENT,
    numero_ticket   INTEGER NOT NULL,
    session         INTEGER NOT NULL REFERENCES session_caisse (identifiant) ON DELETE CASCADE,
    caissier        INTEGER NOT NULL REFERENCES compte (identifiant),
    encaissee_le    TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    total_centimes  INTEGER NOT NULL CHECK (total_centimes >= 0),
    recu_centimes   INTEGER NOT NULL CHECK (recu_centimes >= 0),
    rendu_centimes  INTEGER NOT NULL CHECK (rendu_centimes >= 0),
    nombre_articles INTEGER NOT NULL CHECK (nombre_articles > 0)
);

CREATE INDEX IF NOT EXISTS index_vente_session ON vente (session);
CREATE INDEX IF NOT EXISTS index_vente_caissier ON vente (caissier);
CREATE INDEX IF NOT EXISTS index_vente_date ON vente (encaissee_le);

CREATE TABLE IF NOT EXISTS ligne_vente (
    identifiant    INTEGER PRIMARY KEY AUTOINCREMENT,
    vente          INTEGER NOT NULL REFERENCES vente (identifiant) ON DELETE CASCADE,
    produit        INTEGER REFERENCES produit (identifiant) ON DELETE SET NULL,
    libelle        TEXT    NOT NULL,
    code_barres    TEXT,
    rayon_libelle  TEXT    NOT NULL,
    prix_centimes  INTEGER NOT NULL CHECK (prix_centimes >= 0),
    quantite       INTEGER NOT NULL CHECK (quantite > 0),
    total_centimes INTEGER NOT NULL CHECK (total_centimes >= 0)
);

CREATE INDEX IF NOT EXISTS index_ligne_vente ON ligne_vente (vente);

-- Compteurs persistants : jamais effaces par une remise a zero.
CREATE TABLE IF NOT EXISTS compteur (
    cle    TEXT    PRIMARY KEY,
    valeur INTEGER NOT NULL DEFAULT 0
);

INSERT OR IGNORE INTO compteur (cle, valeur) VALUES
    ('numero_ticket', 0),
    ('remise_a_zero', 0),
    ('grand_total_centimes', 0),
    ('grand_total_articles', 0),
    ('grand_total_ventes', 0);

CREATE TABLE IF NOT EXISTS rapport_z (
    identifiant     INTEGER PRIMARY KEY AUTOINCREMENT,
    numero          INTEGER NOT NULL,
    genere_le       TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    debut_le        TEXT,
    fin_le          TEXT,
    nombre_ventes   INTEGER NOT NULL,
    nombre_articles INTEGER NOT NULL,
    total_centimes  INTEGER NOT NULL,
    premier_ticket  INTEGER,
    dernier_ticket  INTEGER,
    texte           TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS index_rapport_z_numero ON rapport_z (numero);

INSERT OR IGNORE INTO parametre (cle, valeur) VALUES
    ('ticket_actif', 'oui'),
    ('ticket_impression_auto', 'non'),
    ('ticket_largeur_colonnes', '32'),
    ('ticket_entete', 'Merci de votre visite'),
    ('ticket_pied', 'A bientot');
