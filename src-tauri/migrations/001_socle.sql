CREATE TABLE IF NOT EXISTS parametre (
    cle          TEXT PRIMARY KEY,
    valeur       TEXT NOT NULL,
    modifie_le   TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO parametre (cle, valeur) VALUES
    ('devise', 'EUR'),
    ('symbole_devise', 'EUR'),
    ('taux_tva_defaut', '20.00'),
    ('nom_commerce', 'Mon commerce');
