/*
 * Contenu transferable de la caisse : ce qui part dans un fichier d export et
 * ce qui revient d un fichier d import.
 *
 * Deux niveaux, jamais melanges :
 *   - le parametrage : reglages, comptes, rayons, produits. C est la
 *     programmation du commerce, elle se recopie d une caisse a l autre ;
 *   - les donnees : compteurs, sessions, ventes, lignes, rapports Z. C est
 *     l historique du poste, il ne se recopie que pour une restauration.
 *
 * Les identifiants sont conserves tels quels : une ligne de vente doit encore
 * pointer sur son produit apres un aller-retour par le fichier.
 */
use rusqlite::{Connection, Transaction};
use serde::{Deserialize, Serialize};

use crate::erreur::Resultat;

use super::traduction::conflit_si_contrainte;

/// Parametres propres a un poste : nom de l imprimante vue par Windows,
/// signature de la douchette branchee, horodatages de detection. Les recopier
/// sur une autre machine y installerait des reglages faux.
const CLES_LOCALES: &[&str] = &[
    "imprimante_nom",
    "imprimante_detectee",
    "imprimante_derniere_detection",
    "scanner_detecte",
    "scanner_derniere_detection",
    "scanner_signature",
];

#[derive(Debug, Serialize, Deserialize)]
pub struct Parametre {
    pub cle: String,
    pub valeur: String,
}

/// Le code secret ne circule jamais en clair : seul son scellement voyage,
/// donc un compte importe repond au meme code qu avant l export.
#[derive(Debug, Serialize, Deserialize)]
pub struct Compte {
    pub identifiant: i64,
    pub nom: String,
    pub role: String,
    pub empreinte: String,
    pub sel: String,
    pub iterations: i64,
    pub actif: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rayon {
    pub identifiant: i64,
    pub libelle: String,
    pub ordre: i64,
    pub actif: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Produit {
    pub identifiant: i64,
    pub rayon: i64,
    pub libelle: String,
    pub code_barres: Option<String>,
    pub prix_centimes: i64,
    pub stock: i64,
    pub suivi_stock: bool,
    pub actif: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Compteur {
    pub cle: String,
    pub valeur: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub identifiant: i64,
    pub caissier: i64,
    pub ouverte_le: String,
    pub fermee_le: Option<String>,
    pub fond_centimes: i64,
    pub etat: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Vente {
    pub identifiant: i64,
    pub numero_ticket: i64,
    pub session: i64,
    pub caissier: i64,
    pub encaissee_le: String,
    pub total_centimes: i64,
    pub recu_centimes: i64,
    pub rendu_centimes: i64,
    pub nombre_articles: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LigneVente {
    pub identifiant: i64,
    pub vente: i64,
    pub produit: Option<i64>,
    pub libelle: String,
    pub code_barres: Option<String>,
    pub rayon_libelle: String,
    pub prix_centimes: i64,
    pub quantite: i64,
    pub total_centimes: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RapportZ {
    pub identifiant: i64,
    pub numero: i64,
    pub genere_le: String,
    pub debut_le: Option<String>,
    pub fin_le: Option<String>,
    pub nombre_ventes: i64,
    pub nombre_articles: i64,
    pub total_centimes: i64,
    pub premier_ticket: Option<i64>,
    pub dernier_ticket: Option<i64>,
    pub texte: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Parametrage {
    pub parametres: Vec<Parametre>,
    pub comptes: Vec<Compte>,
    pub rayons: Vec<Rayon>,
    pub produits: Vec<Produit>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Donnees {
    pub compteurs: Vec<Compteur>,
    pub sessions: Vec<Session>,
    pub ventes: Vec<Vente>,
    pub lignes: Vec<LigneVente>,
    pub rapports_z: Vec<RapportZ>,
}

/// Ce que le gerant voit avant de cliquer : combien de lignes partent, ou
/// combien de lignes arrivent.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Resume {
    pub parametres: i64,
    pub comptes: i64,
    pub rayons: i64,
    pub produits: i64,
    pub sessions: i64,
    pub ventes: i64,
    pub lignes: i64,
    pub rapports_z: i64,
}

pub fn lire_parametrage(connexion: &Connection) -> Resultat<Parametrage> {
    Ok(Parametrage {
        parametres: lire_parametres(connexion)?,
        comptes: lire_comptes(connexion)?,
        rayons: lire_rayons(connexion)?,
        produits: lire_produits(connexion)?,
    })
}

pub fn lire_donnees(connexion: &Connection) -> Resultat<Donnees> {
    Ok(Donnees {
        compteurs: lire_compteurs(connexion)?,
        sessions: lire_sessions(connexion)?,
        ventes: lire_ventes(connexion)?,
        lignes: lire_lignes(connexion)?,
        rapports_z: lire_rapports_z(connexion)?,
    })
}

/// Remplacement de la programmation. Les ventes et les sessions doivent avoir
/// ete purgees avant : elles pointent sur les comptes et les produits effaces
/// ici, et la base refuse de les laisser orphelines.
pub fn ecrire_parametrage(transaction: &Transaction, contenu: &Parametrage) -> Resultat<()> {
    transaction.execute("DELETE FROM produit", [])?;
    transaction.execute("DELETE FROM rayon", [])?;
    transaction.execute("DELETE FROM compte", [])?;

    for parametre in &contenu.parametres {
        transaction
            .execute(
                "INSERT INTO parametre (cle, valeur) VALUES (?1, ?2)
                 ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur,
                                                modifie_le = datetime('now')",
                (&parametre.cle, &parametre.valeur),
            )
            .map_err(|erreur| {
                conflit_si_contrainte(erreur, "fichier refuse : parametre illisible")
            })?;
    }

    for compte in &contenu.comptes {
        transaction
            .execute(
                "INSERT INTO compte (identifiant, nom, role, empreinte, sel, iterations, actif)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    compte.identifiant,
                    &compte.nom,
                    &compte.role,
                    &compte.empreinte,
                    &compte.sel,
                    compte.iterations,
                    i64::from(compte.actif),
                ),
            )
            .map_err(|erreur| {
                conflit_si_contrainte(
                    erreur,
                    "fichier refuse : deux comptes de meme nom, ou deux gerants",
                )
            })?;
    }

    for rayon in &contenu.rayons {
        transaction
            .execute(
                "INSERT INTO rayon (identifiant, libelle, ordre, actif) VALUES (?1, ?2, ?3, ?4)",
                (
                    rayon.identifiant,
                    &rayon.libelle,
                    rayon.ordre,
                    i64::from(rayon.actif),
                ),
            )
            .map_err(|erreur| {
                conflit_si_contrainte(erreur, "fichier refuse : deux rayons de meme libelle")
            })?;
    }

    for produit in &contenu.produits {
        transaction
            .execute(
                "INSERT INTO produit (identifiant, rayon, libelle, code_barres, prix_centimes,
                                      stock, suivi_stock, actif)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    produit.identifiant,
                    produit.rayon,
                    &produit.libelle,
                    &produit.code_barres,
                    produit.prix_centimes,
                    produit.stock,
                    i64::from(produit.suivi_stock),
                    i64::from(produit.actif),
                ),
            )
            .map_err(|erreur| {
                conflit_si_contrainte(
                    erreur,
                    "fichier refuse : code-barres en double, ou rayon inconnu d un produit",
                )
            })?;
    }

    Ok(())
}

/// Effacement de l historique, dans l ordre des dependances : les lignes avant
/// les ventes, les ventes avant les sessions. A appeler avant d effacer les
/// comptes et les produits, qui sont pointes par ces lignes.
pub fn effacer_historique(transaction: &Transaction) -> Resultat<()> {
    transaction.execute("DELETE FROM ligne_vente", [])?;
    transaction.execute("DELETE FROM vente", [])?;
    transaction.execute("DELETE FROM session_caisse", [])?;
    transaction.execute("DELETE FROM rapport_z", [])?;

    Ok(())
}

/// Pose de l historique importe. Les comptes et les produits doivent deja etre
/// ceux du fichier : une session pointe sur son caissier des son insertion.
pub fn ecrire_donnees(transaction: &Transaction, contenu: &Donnees) -> Resultat<()> {
    for compteur in &contenu.compteurs {
        transaction.execute(
            "INSERT INTO compteur (cle, valeur) VALUES (?1, ?2)
             ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur",
            (&compteur.cle, compteur.valeur),
        )?;
    }

    for session in &contenu.sessions {
        transaction
            .execute(
                "INSERT INTO session_caisse (identifiant, caissier, ouverte_le, fermee_le,
                                             fond_centimes, etat)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (
                    session.identifiant,
                    session.caissier,
                    &session.ouverte_le,
                    &session.fermee_le,
                    session.fond_centimes,
                    &session.etat,
                ),
            )
            .map_err(|erreur| {
                conflit_si_contrainte(
                    erreur,
                    "fichier refuse : session sans caissier, ou deux sessions ouvertes",
                )
            })?;
    }

    for vente in &contenu.ventes {
        transaction
            .execute(
                "INSERT INTO vente (identifiant, numero_ticket, session, caissier, encaissee_le,
                                    total_centimes, recu_centimes, rendu_centimes, nombre_articles)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                (
                    vente.identifiant,
                    vente.numero_ticket,
                    vente.session,
                    vente.caissier,
                    &vente.encaissee_le,
                    vente.total_centimes,
                    vente.recu_centimes,
                    vente.rendu_centimes,
                    vente.nombre_articles,
                ),
            )
            .map_err(|erreur| {
                conflit_si_contrainte(erreur, "fichier refuse : vente sans session ni caissier")
            })?;
    }

    for ligne in &contenu.lignes {
        transaction
            .execute(
                "INSERT INTO ligne_vente (identifiant, vente, produit, libelle, code_barres,
                                          rayon_libelle, prix_centimes, quantite, total_centimes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                (
                    ligne.identifiant,
                    ligne.vente,
                    ligne.produit,
                    &ligne.libelle,
                    &ligne.code_barres,
                    &ligne.rayon_libelle,
                    ligne.prix_centimes,
                    ligne.quantite,
                    ligne.total_centimes,
                ),
            )
            .map_err(|erreur| {
                conflit_si_contrainte(erreur, "fichier refuse : ligne sans vente")
            })?;
    }

    for rapport in &contenu.rapports_z {
        transaction.execute(
            "INSERT INTO rapport_z (identifiant, numero, genere_le, debut_le, fin_le,
                                    nombre_ventes, nombre_articles, total_centimes,
                                    premier_ticket, dernier_ticket, texte)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            (
                rapport.identifiant,
                rapport.numero,
                &rapport.genere_le,
                &rapport.debut_le,
                &rapport.fin_le,
                rapport.nombre_ventes,
                rapport.nombre_articles,
                rapport.total_centimes,
                rapport.premier_ticket,
                rapport.dernier_ticket,
                &rapport.texte,
            ),
        )?;
    }

    Ok(())
}

pub fn resumer(connexion: &Connection) -> Resultat<Resume> {
    Ok(Resume {
        parametres: compter(connexion, "parametre")?,
        comptes: compter(connexion, "compte")?,
        rayons: compter(connexion, "rayon")?,
        produits: compter(connexion, "produit")?,
        sessions: compter(connexion, "session_caisse")?,
        ventes: compter(connexion, "vente")?,
        lignes: compter(connexion, "ligne_vente")?,
        rapports_z: compter(connexion, "rapport_z")?,
    })
}

pub fn resumer_contenu(parametrage: &Parametrage, donnees: Option<&Donnees>) -> Resume {
    let mut resume = Resume {
        parametres: parametrage.parametres.len() as i64,
        comptes: parametrage.comptes.len() as i64,
        rayons: parametrage.rayons.len() as i64,
        produits: parametrage.produits.len() as i64,
        ..Resume::default()
    };

    if let Some(historique) = donnees {
        resume.sessions = historique.sessions.len() as i64;
        resume.ventes = historique.ventes.len() as i64;
        resume.lignes = historique.lignes.len() as i64;
        resume.rapports_z = historique.rapports_z.len() as i64;
    }

    resume
}

fn compter(connexion: &Connection, table: &str) -> Resultat<i64> {
    let total =
        connexion.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |ligne| {
            ligne.get(0)
        })?;

    Ok(total)
}

fn lire_parametres(connexion: &Connection) -> Resultat<Vec<Parametre>> {
    let mut requete = connexion.prepare("SELECT cle, valeur FROM parametre ORDER BY cle")?;
    let mut lignes = requete.query([])?;
    let mut parametres = Vec::new();

    while let Some(ligne) = lignes.next()? {
        let cle: String = ligne.get(0)?;

        if CLES_LOCALES.contains(&cle.as_str()) {
            continue;
        }

        parametres.push(Parametre {
            cle,
            valeur: ligne.get(1)?,
        });
    }

    Ok(parametres)
}

fn lire_comptes(connexion: &Connection) -> Resultat<Vec<Compte>> {
    let mut requete = connexion.prepare(
        "SELECT identifiant, nom, role, empreinte, sel, iterations, actif
           FROM compte ORDER BY identifiant",
    )?;
    let mut lignes = requete.query([])?;
    let mut comptes = Vec::new();

    while let Some(ligne) = lignes.next()? {
        comptes.push(Compte {
            identifiant: ligne.get(0)?,
            nom: ligne.get(1)?,
            role: ligne.get(2)?,
            empreinte: ligne.get(3)?,
            sel: ligne.get(4)?,
            iterations: ligne.get(5)?,
            actif: ligne.get::<_, i64>(6)? == 1,
        });
    }

    Ok(comptes)
}

fn lire_rayons(connexion: &Connection) -> Resultat<Vec<Rayon>> {
    let mut requete = connexion
        .prepare("SELECT identifiant, libelle, ordre, actif FROM rayon ORDER BY identifiant")?;
    let mut lignes = requete.query([])?;
    let mut rayons = Vec::new();

    while let Some(ligne) = lignes.next()? {
        rayons.push(Rayon {
            identifiant: ligne.get(0)?,
            libelle: ligne.get(1)?,
            ordre: ligne.get(2)?,
            actif: ligne.get::<_, i64>(3)? == 1,
        });
    }

    Ok(rayons)
}

fn lire_produits(connexion: &Connection) -> Resultat<Vec<Produit>> {
    let mut requete = connexion.prepare(
        "SELECT identifiant, rayon, libelle, code_barres, prix_centimes, stock, suivi_stock, actif
           FROM produit ORDER BY identifiant",
    )?;
    let mut lignes = requete.query([])?;
    let mut produits = Vec::new();

    while let Some(ligne) = lignes.next()? {
        produits.push(Produit {
            identifiant: ligne.get(0)?,
            rayon: ligne.get(1)?,
            libelle: ligne.get(2)?,
            code_barres: ligne.get(3)?,
            prix_centimes: ligne.get(4)?,
            stock: ligne.get(5)?,
            suivi_stock: ligne.get::<_, i64>(6)? == 1,
            actif: ligne.get::<_, i64>(7)? == 1,
        });
    }

    Ok(produits)
}

fn lire_compteurs(connexion: &Connection) -> Resultat<Vec<Compteur>> {
    let mut requete = connexion.prepare("SELECT cle, valeur FROM compteur ORDER BY cle")?;
    let mut lignes = requete.query([])?;
    let mut compteurs = Vec::new();

    while let Some(ligne) = lignes.next()? {
        compteurs.push(Compteur {
            cle: ligne.get(0)?,
            valeur: ligne.get(1)?,
        });
    }

    Ok(compteurs)
}

fn lire_sessions(connexion: &Connection) -> Resultat<Vec<Session>> {
    let mut requete = connexion.prepare(
        "SELECT identifiant, caissier, ouverte_le, fermee_le, fond_centimes, etat
           FROM session_caisse ORDER BY identifiant",
    )?;
    let mut lignes = requete.query([])?;
    let mut sessions = Vec::new();

    while let Some(ligne) = lignes.next()? {
        sessions.push(Session {
            identifiant: ligne.get(0)?,
            caissier: ligne.get(1)?,
            ouverte_le: ligne.get(2)?,
            fermee_le: ligne.get(3)?,
            fond_centimes: ligne.get(4)?,
            etat: ligne.get(5)?,
        });
    }

    Ok(sessions)
}

fn lire_ventes(connexion: &Connection) -> Resultat<Vec<Vente>> {
    let mut requete = connexion.prepare(
        "SELECT identifiant, numero_ticket, session, caissier, encaissee_le, total_centimes,
                recu_centimes, rendu_centimes, nombre_articles
           FROM vente ORDER BY identifiant",
    )?;
    let mut lignes = requete.query([])?;
    let mut ventes = Vec::new();

    while let Some(ligne) = lignes.next()? {
        ventes.push(Vente {
            identifiant: ligne.get(0)?,
            numero_ticket: ligne.get(1)?,
            session: ligne.get(2)?,
            caissier: ligne.get(3)?,
            encaissee_le: ligne.get(4)?,
            total_centimes: ligne.get(5)?,
            recu_centimes: ligne.get(6)?,
            rendu_centimes: ligne.get(7)?,
            nombre_articles: ligne.get(8)?,
        });
    }

    Ok(ventes)
}

fn lire_lignes(connexion: &Connection) -> Resultat<Vec<LigneVente>> {
    let mut requete = connexion.prepare(
        "SELECT identifiant, vente, produit, libelle, code_barres, rayon_libelle, prix_centimes,
                quantite, total_centimes
           FROM ligne_vente ORDER BY identifiant",
    )?;
    let mut lignes = requete.query([])?;
    let mut resultat = Vec::new();

    while let Some(ligne) = lignes.next()? {
        resultat.push(LigneVente {
            identifiant: ligne.get(0)?,
            vente: ligne.get(1)?,
            produit: ligne.get(2)?,
            libelle: ligne.get(3)?,
            code_barres: ligne.get(4)?,
            rayon_libelle: ligne.get(5)?,
            prix_centimes: ligne.get(6)?,
            quantite: ligne.get(7)?,
            total_centimes: ligne.get(8)?,
        });
    }

    Ok(resultat)
}

fn lire_rapports_z(connexion: &Connection) -> Resultat<Vec<RapportZ>> {
    let mut requete = connexion.prepare(
        "SELECT identifiant, numero, genere_le, debut_le, fin_le, nombre_ventes, nombre_articles,
                total_centimes, premier_ticket, dernier_ticket, texte
           FROM rapport_z ORDER BY identifiant",
    )?;
    let mut lignes = requete.query([])?;
    let mut rapports = Vec::new();

    while let Some(ligne) = lignes.next()? {
        rapports.push(RapportZ {
            identifiant: ligne.get(0)?,
            numero: ligne.get(1)?,
            genere_le: ligne.get(2)?,
            debut_le: ligne.get(3)?,
            fin_le: ligne.get(4)?,
            nombre_ventes: ligne.get(5)?,
            nombre_articles: ligne.get(6)?,
            total_centimes: ligne.get(7)?,
            premier_ticket: ligne.get(8)?,
            dernier_ticket: ligne.get(9)?,
            texte: ligne.get(10)?,
        });
    }

    Ok(rapports)
}
