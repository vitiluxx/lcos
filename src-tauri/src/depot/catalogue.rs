use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::erreur::{ErreurApplication, Resultat};

use super::traduction::conflit_si_contrainte;

pub const LONGUEUR_LIBELLE_MINIMALE: usize = 2;
pub const LONGUEUR_LIBELLE_MAXIMALE: usize = 60;
pub const LIMITE_LISTE: i64 = 500;

#[derive(Debug, Serialize)]
pub struct Rayon {
    pub identifiant: i64,
    pub libelle: String,
    pub ordre: i64,
    pub actif: bool,
    pub nombre_produits: i64,
}

#[derive(Debug, Serialize)]
pub struct Produit {
    pub identifiant: i64,
    pub rayon: i64,
    pub rayon_libelle: String,
    pub libelle: String,
    pub code_barres: Option<String>,
    pub prix_centimes: i64,
    pub stock: i64,
    pub suivi_stock: bool,
    pub actif: bool,
}

#[derive(Debug, Default)]
pub struct Filtre {
    pub recherche: Option<String>,
    pub rayon: Option<i64>,
    pub inclure_inactifs: bool,
}

const SELECTION_RAYON: &str = "SELECT rayon.identifiant,
                                      rayon.libelle,
                                      rayon.ordre,
                                      rayon.actif,
                                      (SELECT COUNT(*) FROM produit WHERE produit.rayon = rayon.identifiant)
                                 FROM rayon";

const SELECTION_PRODUIT: &str = "SELECT produit.identifiant,
                                        produit.rayon,
                                        rayon.libelle,
                                        produit.libelle,
                                        produit.code_barres,
                                        produit.prix_centimes,
                                        produit.stock,
                                        produit.suivi_stock,
                                        produit.actif
                                   FROM produit
                                   JOIN rayon ON rayon.identifiant = produit.rayon";

pub fn valider_libelle(libelle: &str) -> Resultat<String> {
    let nettoye = libelle.trim();

    if nettoye.len() < LONGUEUR_LIBELLE_MINIMALE || nettoye.len() > LONGUEUR_LIBELLE_MAXIMALE {
        return Err(ErreurApplication::Validation(format!(
            "libelle : de {LONGUEUR_LIBELLE_MINIMALE} a {LONGUEUR_LIBELLE_MAXIMALE} caracteres"
        )));
    }

    if nettoye.chars().any(|caractere| caractere.is_control()) {
        return Err(ErreurApplication::Validation(
            "libelle : caracteres de controle refuses".to_string(),
        ));
    }

    Ok(nettoye.to_string())
}

fn lire_rayon(ligne: &rusqlite::Row) -> rusqlite::Result<Rayon> {
    Ok(Rayon {
        identifiant: ligne.get(0)?,
        libelle: ligne.get(1)?,
        ordre: ligne.get(2)?,
        actif: ligne.get::<_, i64>(3)? == 1,
        nombre_produits: ligne.get(4)?,
    })
}

fn lire_produit(ligne: &rusqlite::Row) -> rusqlite::Result<Produit> {
    Ok(Produit {
        identifiant: ligne.get(0)?,
        rayon: ligne.get(1)?,
        rayon_libelle: ligne.get(2)?,
        libelle: ligne.get(3)?,
        code_barres: ligne.get(4)?,
        prix_centimes: ligne.get(5)?,
        stock: ligne.get(6)?,
        suivi_stock: ligne.get::<_, i64>(7)? == 1,
        actif: ligne.get::<_, i64>(8)? == 1,
    })
}

pub fn lister_rayons(connexion: &Connection, inclure_inactifs: bool) -> Resultat<Vec<Rayon>> {
    let condition = if inclure_inactifs {
        ""
    } else {
        " WHERE rayon.actif = 1"
    };

    let mut requete = connexion.prepare(&format!(
        "{SELECTION_RAYON}{condition} ORDER BY rayon.ordre, rayon.libelle"
    ))?;
    let mut lignes = requete.query([])?;
    let mut rayons = Vec::new();

    while let Some(ligne) = lignes.next()? {
        rayons.push(lire_rayon(ligne)?);
    }

    Ok(rayons)
}

pub fn exiger_rayon(connexion: &Connection, identifiant: i64) -> Resultat<Rayon> {
    connexion
        .query_row(
            &format!("{SELECTION_RAYON} WHERE rayon.identifiant = ?1"),
            [identifiant],
            lire_rayon,
        )
        .optional()?
        .ok_or_else(|| ErreurApplication::Introuvable(format!("rayon {identifiant}")))
}

pub fn creer_rayon(connexion: &Connection, libelle: &str, ordre: i64) -> Resultat<i64> {
    connexion
        .execute(
            "INSERT INTO rayon (libelle, ordre) VALUES (?1, ?2)",
            (libelle, ordre),
        )
        .map_err(|erreur| {
            conflit_si_contrainte(erreur, &format!("le rayon {libelle} existe deja"))
        })?;

    Ok(connexion.last_insert_rowid())
}

pub fn modifier_rayon(
    connexion: &Connection,
    identifiant: i64,
    libelle: &str,
    ordre: i64,
    actif: bool,
) -> Resultat<()> {
    let touchees = connexion
        .execute(
            "UPDATE rayon SET libelle = ?2, ordre = ?3, actif = ?4,
                              modifie_le = datetime('now', 'localtime')
              WHERE identifiant = ?1",
            (identifiant, libelle, ordre, i64::from(actif)),
        )
        .map_err(|erreur| {
            conflit_si_contrainte(erreur, &format!("le rayon {libelle} existe deja"))
        })?;

    if touchees == 0 {
        return Err(ErreurApplication::Introuvable(format!(
            "rayon {identifiant}"
        )));
    }

    Ok(())
}

pub fn supprimer_rayon(connexion: &Connection, identifiant: i64) -> Resultat<()> {
    let touchees = connexion
        .execute("DELETE FROM rayon WHERE identifiant = ?1", [identifiant])
        .map_err(|erreur| {
            conflit_si_contrainte(
                erreur,
                "ce rayon contient des produits : vide-le ou desactive-le",
            )
        })?;

    if touchees == 0 {
        return Err(ErreurApplication::Introuvable(format!(
            "rayon {identifiant}"
        )));
    }

    Ok(())
}

pub fn lister_produits(connexion: &Connection, filtre: &Filtre) -> Resultat<Vec<Produit>> {
    let mut conditions: Vec<String> = Vec::new();

    if !filtre.inclure_inactifs {
        conditions.push("produit.actif = 1".to_string());
    }

    if filtre.rayon.is_some() {
        conditions.push("produit.rayon = :rayon".to_string());
    }

    if filtre.recherche.is_some() {
        conditions.push(
            "(produit.libelle LIKE :motif OR produit.code_barres LIKE :motif)".to_string(),
        );
    }

    let filtrage = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let motif = filtre
        .recherche
        .as_ref()
        .map(|texte| format!("%{}%", texte.trim()));

    let mut requete = connexion.prepare(&format!(
        "{SELECTION_PRODUIT}{filtrage} ORDER BY rayon.ordre, produit.libelle LIMIT {LIMITE_LISTE}"
    ))?;

    let mut parametres: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();

    if let Some(rayon) = filtre.rayon.as_ref() {
        parametres.push((":rayon", rayon));
    }

    if let Some(motif) = motif.as_ref() {
        parametres.push((":motif", motif));
    }

    let mut lignes = requete.query(parametres.as_slice())?;
    let mut produits = Vec::new();

    while let Some(ligne) = lignes.next()? {
        produits.push(lire_produit(ligne)?);
    }

    Ok(produits)
}

pub fn lire_produit_par_identifiant(
    connexion: &Connection,
    identifiant: i64,
) -> Resultat<Option<Produit>> {
    let produit = connexion
        .query_row(
            &format!("{SELECTION_PRODUIT} WHERE produit.identifiant = ?1"),
            [identifiant],
            lire_produit,
        )
        .optional()?;

    Ok(produit)
}

pub fn exiger_produit(connexion: &Connection, identifiant: i64) -> Resultat<Produit> {
    lire_produit_par_identifiant(connexion, identifiant)?
        .ok_or_else(|| ErreurApplication::Introuvable(format!("produit {identifiant}")))
}

pub fn trouver_par_code(connexion: &Connection, code: &str) -> Resultat<Option<Produit>> {
    let produit = connexion
        .query_row(
            &format!("{SELECTION_PRODUIT} WHERE produit.code_barres = ?1"),
            [code],
            lire_produit,
        )
        .optional()?;

    Ok(produit)
}

pub fn creer_produit(
    connexion: &Connection,
    rayon: i64,
    libelle: &str,
    code_barres: Option<&str>,
    prix_centimes: i64,
    stock: i64,
    suivi_stock: bool,
) -> Resultat<i64> {
    connexion
        .execute(
            "INSERT INTO produit (rayon, libelle, code_barres, prix_centimes, stock, suivi_stock)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                rayon,
                libelle,
                code_barres,
                prix_centimes,
                stock,
                i64::from(suivi_stock),
            ),
        )
        .map_err(|erreur| {
            conflit_si_contrainte(erreur, "code-barres deja attribue a un autre produit")
        })?;

    Ok(connexion.last_insert_rowid())
}

#[allow(clippy::too_many_arguments)]
pub fn modifier_produit(
    connexion: &Connection,
    identifiant: i64,
    rayon: i64,
    libelle: &str,
    code_barres: Option<&str>,
    prix_centimes: i64,
    stock: i64,
    suivi_stock: bool,
    actif: bool,
) -> Resultat<()> {
    let touchees = connexion
        .execute(
            "UPDATE produit SET rayon = ?2, libelle = ?3, code_barres = ?4, prix_centimes = ?5,
                                stock = ?6, suivi_stock = ?7, actif = ?8,
                                modifie_le = datetime('now', 'localtime')
              WHERE identifiant = ?1",
            (
                identifiant,
                rayon,
                libelle,
                code_barres,
                prix_centimes,
                stock,
                i64::from(suivi_stock),
                i64::from(actif),
            ),
        )
        .map_err(|erreur| {
            conflit_si_contrainte(erreur, "code-barres deja attribue a un autre produit")
        })?;

    if touchees == 0 {
        return Err(ErreurApplication::Introuvable(format!(
            "produit {identifiant}"
        )));
    }

    Ok(())
}

pub fn supprimer_produit(connexion: &Connection, identifiant: i64) -> Resultat<()> {
    let touchees = connexion.execute("DELETE FROM produit WHERE identifiant = ?1", [identifiant])?;

    if touchees == 0 {
        return Err(ErreurApplication::Introuvable(format!(
            "produit {identifiant}"
        )));
    }

    Ok(())
}

/// Decremente le stock des produits suivis. Un stock negatif est accepte :
/// la caisse ne doit jamais bloquer une vente, l inventaire se corrige apres.
pub fn retirer_du_stock(connexion: &Connection, produit: i64, quantite: i64) -> Resultat<()> {
    connexion.execute(
        "UPDATE produit SET stock = stock - ?2, modifie_le = datetime('now', 'localtime')
          WHERE identifiant = ?1 AND suivi_stock = 1",
        (produit, quantite),
    )?;

    Ok(())
}
