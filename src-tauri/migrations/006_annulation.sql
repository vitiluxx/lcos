-- Annulation d une vente deja encaissee. Fermee par defaut : une caisse ou
-- n importe qui peut effacer une vente est une caisse sans comptabilite. Le
-- gerant ouvre la porte en connaissance de cause.
INSERT OR IGNORE INTO parametre (cle, valeur) VALUES ('annulation_vente', 'non');

-- Trace des ventes annulees. La vente, elle, disparait du journal : sans cette
-- trace, rien ne distinguerait une vente annulee d une vente jamais faite.
-- Elle appartient a la periode, donc la remise a zero l emporte avec elle.
CREATE TABLE IF NOT EXISTS vente_annulee (
    identifiant     INTEGER PRIMARY KEY AUTOINCREMENT,
    numero_ticket   INTEGER NOT NULL,
    caissier_nom    TEXT    NOT NULL,
    encaissee_le    TEXT    NOT NULL,
    annulee_le      TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    annulee_par     TEXT    NOT NULL,
    nombre_articles INTEGER NOT NULL,
    total_centimes  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS index_vente_annulee_ticket ON vente_annulee (numero_ticket);
