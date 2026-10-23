use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::domaine::horodatage;
use crate::domaine::ticket::{LigneTicket, TotalNomme, VenteResumee};
use crate::erreur::{ErreurApplication, Resultat};

pub const LIMITE_JOURNAL: i64 = 200;

#[derive(Debug)]
pub struct LigneAEnregistrer {
    pub produit: i64,
    pub libelle: String,
    pub code_barres: Option<String>,
    pub rayon_libelle: String,
    pub prix_centimes: i64,
    pub quantite: i64,
    pub total_centimes: i64,
}

#[derive(Debug)]
pub struct VenteAEnregistrer {
    pub numero_ticket: i64,
    pub session: i64,
    pub caissier: i64,
    pub total_centimes: i64,
    pub recu_centimes: i64,
    pub rendu_centimes: i64,
    pub nombre_articles: i64,
}

#[derive(Debug, Serialize)]
pub struct VenteJournal {
    pub identifiant: i64,
    pub numero_ticket: i64,
    pub horodatage: String,
    pub caissier: String,
    pub caissier_identifiant: i64,
    pub session: i64,
    pub nombre_articles: i64,
    pub total_centimes: i64,
    pub recu_centimes: i64,
    pub rendu_centimes: i64,
}

#[derive(Debug, Default, Serialize)]
pub struct Agregats {
    pub nombre_ventes: i64,
    pub nombre_articles: i64,
    pub total_centimes: i64,
    pub premier_ticket: i64,
    pub dernier_ticket: i64,
    pub debut_le: String,
    pub fin_le: String,
}

pub fn enregistrer(
    connexion: &Connection,
    vente: &VenteAEnregistrer,
    lignes: &[LigneAEnregistrer],
) -> Resultat<i64> {
    connexion.execute(
        "INSERT INTO vente (numero_ticket, session, caissier, total_centimes,
                            recu_centimes, rendu_centimes, nombre_articles)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            vente.numero_ticket,
            vente.session,
            vente.caissier,
            vente.total_centimes,
            vente.recu_centimes,
            vente.rendu_centimes,
            vente.nombre_articles,
        ),
    )?;

    let identifiant = connexion.last_insert_rowid();

    let mut insertion = connexion.prepare(
        "INSERT INTO ligne_vente (vente, produit, libelle, code_barres, rayon_libelle,
                                  prix_centimes, quantite, total_centimes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;

    for ligne in lignes {
        insertion.execute((
            identifiant,
            ligne.produit,
            &ligne.libelle,
            &ligne.code_barres,
            &ligne.rayon_libelle,
            ligne.prix_centimes,
            ligne.quantite,
            ligne.total_centimes,
        ))?;
    }

    Ok(identifiant)
}

pub fn lignes_de(connexion: &Connection, vente: i64) -> Resultat<Vec<LigneTicket>> {
    let mut requete = connexion.prepare(
        "SELECT libelle, quantite, prix_centimes, total_centimes
           FROM ligne_vente WHERE vente = ?1 ORDER BY identifiant",
    )?;
    let mut lignes = requete.query([vente])?;
    let mut resultat = Vec::new();

    while let Some(ligne) = lignes.next()? {
        resultat.push(LigneTicket {
            libelle: ligne.get(0)?,
            quantite: ligne.get(1)?,
            prix_centimes: ligne.get(2)?,
            total_centimes: ligne.get(3)?,
        });
    }

    Ok(resultat)
}

const SELECTION_JOURNAL: &str = "SELECT vente.identifiant,
                                        vente.numero_ticket,
                                        vente.encaissee_le,
                                        compte.nom,
                                        vente.caissier,
                                        vente.session,
                                        vente.nombre_articles,
                                        vente.total_centimes,
                                        vente.recu_centimes,
                                        vente.rendu_centimes
                                   FROM vente
                                   JOIN compte ON compte.identifiant = vente.caissier";

fn lire_journal(ligne: &rusqlite::Row) -> rusqlite::Result<VenteJournal> {
    let brut: String = ligne.get(2)?;

    Ok(VenteJournal {
        identifiant: ligne.get(0)?,
        numero_ticket: ligne.get(1)?,
        horodatage: horodatage::lisible(&brut),
        caissier: ligne.get(3)?,
        caissier_identifiant: ligne.get(4)?,
        session: ligne.get(5)?,
        nombre_articles: ligne.get(6)?,
        total_centimes: ligne.get(7)?,
        recu_centimes: ligne.get(8)?,
        rendu_centimes: ligne.get(9)?,
    })
}

pub fn journal(connexion: &Connection, caissier: Option<i64>) -> Resultat<Vec<VenteJournal>> {
    let filtrage = if caissier.is_some() {
        " WHERE vente.caissier = ?1"
    } else {
        ""
    };

    let mut requete = connexion.prepare(&format!(
        "{SELECTION_JOURNAL}{filtrage} ORDER BY vente.identifiant DESC LIMIT {LIMITE_JOURNAL}"
    ))?;

    let mut lignes = match caissier {
        Some(identifiant) => requete.query([identifiant])?,
        None => requete.query([])?,
    };

    let mut resultat = Vec::new();

    while let Some(ligne) = lignes.next()? {
        resultat.push(lire_journal(ligne)?);
    }

    Ok(resultat)
}

pub fn exiger(connexion: &Connection, identifiant: i64) -> Resultat<VenteJournal> {
    connexion
        .query_row(
            &format!("{SELECTION_JOURNAL} WHERE vente.identifiant = ?1"),
            [identifiant],
            lire_journal,
        )
        .optional()?
        .ok_or_else(|| ErreurApplication::Introuvable(format!("vente {identifiant}")))
}

pub fn resumes(connexion: &Connection) -> Resultat<Vec<VenteResumee>> {
    let mut requete = connexion.prepare(
        "SELECT vente.numero_ticket,
                vente.encaissee_le,
                compte.nom,
                vente.nombre_articles,
                vente.total_centimes
           FROM vente
           JOIN compte ON compte.identifiant = vente.caissier
          ORDER BY vente.numero_ticket",
    )?;
    let mut lignes = requete.query([])?;
    let mut resultat = Vec::new();

    while let Some(ligne) = lignes.next()? {
        let brut: String = ligne.get(1)?;

        resultat.push(VenteResumee {
            numero_ticket: ligne.get(0)?,
            horodatage: horodatage::abreger(&brut),
            caissier: ligne.get(2)?,
            nombre_articles: ligne.get(3)?,
            total_centimes: ligne.get(4)?,
        });
    }

    Ok(resultat)
}

pub fn agregats(connexion: &Connection) -> Resultat<Agregats> {
    let agregats = connexion.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(nombre_articles), 0),
                COALESCE(SUM(total_centimes), 0),
                COALESCE(MIN(numero_ticket), 0),
                COALESCE(MAX(numero_ticket), 0),
                COALESCE(MIN(encaissee_le), ''),
                COALESCE(MAX(encaissee_le), '')
           FROM vente",
        [],
        |ligne| {
            let debut: String = ligne.get(5)?;
            let fin: String = ligne.get(6)?;

            Ok(Agregats {
                nombre_ventes: ligne.get(0)?,
                nombre_articles: ligne.get(1)?,
                total_centimes: ligne.get(2)?,
                premier_ticket: ligne.get(3)?,
                dernier_ticket: ligne.get(4)?,
                debut_le: horodatage::lisible(&debut),
                fin_le: horodatage::lisible(&fin),
            })
        },
    )?;

    Ok(agregats)
}

pub fn totaux_par_caissier(connexion: &Connection) -> Resultat<Vec<TotalNomme>> {
    let mut requete = connexion.prepare(
        "SELECT compte.nom, COUNT(*), COALESCE(SUM(vente.total_centimes), 0)
           FROM vente
           JOIN compte ON compte.identifiant = vente.caissier
          GROUP BY compte.identifiant
          ORDER BY compte.nom",
    )?;

    collecter_totaux(&mut requete)
}

pub fn totaux_par_rayon(connexion: &Connection) -> Resultat<Vec<TotalNomme>> {
    let mut requete = connexion.prepare(
        "SELECT rayon_libelle, COALESCE(SUM(quantite), 0), COALESCE(SUM(total_centimes), 0)
           FROM ligne_vente
          GROUP BY rayon_libelle
          ORDER BY rayon_libelle",
    )?;

    collecter_totaux(&mut requete)
}

fn collecter_totaux(requete: &mut rusqlite::Statement) -> Resultat<Vec<TotalNomme>> {
    let mut lignes = requete.query([])?;
    let mut resultat = Vec::new();

    while let Some(ligne) = lignes.next()? {
        resultat.push(TotalNomme {
            libelle: ligne.get(0)?,
            quantite: ligne.get(1)?,
            total_centimes: ligne.get(2)?,
        });
    }

    Ok(resultat)
}

/// Efface la memoire des ventes. Les compteurs persistants et toute la
/// programmation (rayons, produits, comptes) restent intacts.
pub fn purger(connexion: &Connection) -> Resultat<usize> {
    connexion.execute("DELETE FROM ligne_vente", [])?;

    let supprimees = connexion.execute("DELETE FROM vente", [])?;

    Ok(supprimees)
}
