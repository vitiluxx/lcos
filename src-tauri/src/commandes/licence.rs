/*
 * Droit d usage de LCOS : essai gratuit, puis licence signee.
 *
 * Politique, en clair :
 *   - une caisse neuve fonctionne entierement pendant SEPT jours, sans rien
 *     saisir : le client installe, programme, vend, juge ;
 *   - l essai epuise, la vente est fermee jusqu a l insertion d une licence.
 *     Rien n est efface : la programmation et les ventes attendent sur place ;
 *   - une licence valide rouvre tout immediatement, hors ligne.
 *
 * Le decompte de l essai est monotone : LCOS retient la date la plus avancee
 * qu il ait jamais vue. Reculer l horloge du poste ne rend pas un jour
 * d essai, et c est volontaire sur un logiciel qui sait regler cette horloge.
 */
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

use crate::depot::{horloge, parametres, Depot};
use crate::domaine::{horodatage, licence};
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::RegistreJetons;

const CLE_LICENCE: &str = "licence_texte";
const CLE_ESSAI_DEBUT: &str = "essai_debut";
const CLE_ESSAI_DERNIER_JOUR: &str = "essai_dernier_jour";
const CLE_POSTE: &str = "poste_empreinte";

/// Duree de l essai gratuit, en jours pleins.
pub const JOURS_ESSAI: i64 = 7;

/*
 * Plafonds de l essai gratuit.
 *
 * L essai sert a juger le logiciel sur une vraie caisse : une boutique
 * complete s y programme en entier, et plus rien ne pousse a prendre une
 * licence. Un caissier, un rayon et six produits suffisent a tout essayer,
 * ventes et rapports compris, sans faire tourner un commerce.
 *
 * Ces plafonds sont tenus ici, dans le noyau. Le front grise les boutons par
 * courtoisie : c est cette fonction qui refuse.
 */
pub const ESSAI_CAISSIERS: i64 = 1;
pub const ESSAI_RAYONS: i64 = 1;
pub const ESSAI_PRODUITS: i64 = 6;

const OCTETS_EMPREINTE: usize = 8;

pub const STATUT_ESSAI: &str = "essai";
pub const STATUT_SOUS_LICENCE: &str = "sous_licence";
pub const STATUT_ESSAI_TERMINE: &str = "essai_termine";
pub const STATUT_EXPIREE: &str = "licence_expiree";
pub const STATUT_AUTRE_POSTE: &str = "licence_autre_poste";
pub const STATUT_REFUSEE: &str = "licence_refusee";

#[derive(Debug, Serialize)]
pub struct EtatLicence {
    /// Faux : la vente est fermee, le front n affiche plus que l ecran de
    /// licence et le noyau refuse d encaisser.
    pub autorise: bool,
    pub statut: String,
    pub message: String,
    /// Jours restants d essai, ou de validite d une licence a terme.
    pub jours_restants: i64,
    pub jours_essai: i64,
    /// Empreinte de ce poste, a dicter a l editeur pour une licence liee.
    pub poste: String,
    pub titulaire: String,
    pub numero: String,
    pub emise_le: String,
    pub expire_le: String,
    pub perpetuelle: bool,
    pub sous_licence: bool,
}

#[tauri::command]
pub fn recuperer_etat_licence(depot: State<Depot>) -> Resultat<EtatLicence> {
    depot.executer(evaluer)
}

/// Insertion d une licence. Volontairement sans jeton du gerant : quand
/// l essai est fini, le parametrage n est plus accessible, et une licence est
/// deja une preuve en elle-meme puisqu elle porte la signature de l editeur.
#[tauri::command]
pub fn activer_licence(depot: State<Depot>, cle: String) -> Resultat<EtatLicence> {
    let charge = licence::verifier(&cle)?;
    let compacte: String = cle.chars().filter(|c| !c.is_whitespace()).collect();

    depot.executer(|connexion| {
        if let Some(poste_impose) = charge.poste.as_deref() {
            let local = poste(connexion)?;

            if !licence::meme_poste(poste_impose, &local) {
                return Err(ErreurApplication::AccesRefuse(format!(
                    "licence emise pour le poste {poste_impose} : ce poste est le {}",
                    licence::formater_poste(&local)
                )));
            }
        }

        let aujourd_hui = horloge::maintenant(connexion)?;

        if let Some(fin) = charge.expire_le.as_deref() {
            if jours_restants(&aujourd_hui, fin) < 0 {
                return Err(ErreurApplication::AccesRefuse(format!(
                    "licence expiree le {fin} : demande une licence a jour a l editeur"
                )));
            }
        }

        parametres::ecrire(connexion, CLE_LICENCE, &compacte)?;

        evaluer(connexion)
    })
}

#[tauri::command]
pub fn retirer_licence(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<EtatLicence> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    depot.executer(|connexion| {
        parametres::ecrire(connexion, CLE_LICENCE, "")?;

        evaluer(connexion)
    })
}

/// Garde posee devant les commandes qui font vivre la caisse. Le front masque
/// deja tout, mais c est ici que la fermeture est reelle.
pub fn exiger_active(connexion: &Connection) -> Resultat<()> {
    let etat = evaluer(connexion)?;

    if etat.autorise {
        return Ok(());
    }

    Err(ErreurApplication::AccesRefuse(etat.message))
}

/// Vrai quand une licence valide ouvre ce poste.
///
/// Le drapeau `sous_licence` de l etat dit seulement qu une licence est
/// posee : une licence illisible, expiree ou emise pour une autre caisse le
/// porte aussi. Elle ne doit lever aucun plafond, sinon n importe quel texte
/// dans le reglage vaudrait la version complete.
pub fn sous_licence(connexion: &Connection) -> Resultat<bool> {
    let etat = evaluer(connexion)?;

    Ok(etat.sous_licence && etat.autorise)
}

/// Refuse une creation de plus quand l essai a rempli son quota. Une caisse
/// sous licence n a aucun plafond.
pub fn exiger_quota_essai(
    connexion: &Connection,
    objets: &str,
    presents: i64,
    maximum: i64,
) -> Resultat<()> {
    if sous_licence(connexion)? || presents < maximum {
        return Ok(());
    }

    Err(ErreurApplication::AccesRefuse(format!(
        "version d essai : {maximum} {objets} au maximum. Insere une licence pour aller au dela."
    )))
}

/// Fonctions reservees a une caisse sous licence. L export et l import
/// recopient une programmation entiere d un poste a l autre : pendant l essai,
/// ce serait le moyen de peupler une caisse sans licence en un seul fichier.
pub fn exiger_sous_licence(connexion: &Connection, quoi: &str) -> Resultat<()> {
    if sous_licence(connexion)? {
        return Ok(());
    }

    Err(ErreurApplication::AccesRefuse(format!(
        "version d essai : {quoi} demande une licence. Parametrage --> Licence."
    )))
}

/// Etat du droit d usage. Ecrit au passage le debut de l essai, le jour le
/// plus avance vu et l empreinte du poste : trois valeurs posees une fois pour
/// toutes, au premier demarrage.
pub fn evaluer(connexion: &Connection) -> Resultat<EtatLicence> {
    let maintenant = horloge::maintenant(connexion)?;
    let empreinte = licence::formater_poste(&poste(connexion)?);
    let stockee = parametres::lire_ou(connexion, CLE_LICENCE, "")?;

    if !stockee.trim().is_empty() {
        return etat_sous_licence(connexion, &stockee, &maintenant, empreinte);
    }

    let jour = jour_de_reference(connexion, &maintenant)?;
    let debut = debut_essai(connexion, &maintenant)?;
    let consommes = horodatage::jours_entre(&debut, &jour).unwrap_or(JOURS_ESSAI);
    let restants = JOURS_ESSAI - consommes;

    let fin_essai = horodatage::date_depuis_epoque(
        horodatage::jours_depuis_epoque(&debut).unwrap_or(0) + JOURS_ESSAI,
    );

    if restants > 0 {
        return Ok(EtatLicence {
            autorise: true,
            statut: STATUT_ESSAI.to_string(),
            message: format!(
                "Essai gratuit : {restants} jour(s) restant(s) sur {JOURS_ESSAI}."
            ),
            jours_restants: restants,
            jours_essai: JOURS_ESSAI,
            poste: empreinte,
            titulaire: String::new(),
            numero: String::new(),
            emise_le: debut,
            expire_le: fin_essai,
            perpetuelle: false,
            sous_licence: false,
        });
    }

    Ok(EtatLicence {
        autorise: false,
        statut: STATUT_ESSAI_TERMINE.to_string(),
        message: format!(
            "Essai gratuit de {JOURS_ESSAI} jours termine. Insere une licence pour rouvrir la caisse."
        ),
        jours_restants: 0,
        jours_essai: JOURS_ESSAI,
        poste: empreinte,
        titulaire: String::new(),
        numero: String::new(),
        emise_le: debut,
        expire_le: String::new(),
        perpetuelle: false,
        sous_licence: false,
    })
}

fn etat_sous_licence(
    connexion: &Connection,
    stockee: &str,
    maintenant: &str,
    empreinte: String,
) -> Resultat<EtatLicence> {
    let charge = match licence::verifier(stockee) {
        Ok(charge) => charge,
        Err(incident) => {
            return Ok(EtatLicence {
                autorise: false,
                statut: STATUT_REFUSEE.to_string(),
                message: format!("Licence illisible : {incident}"),
                jours_restants: 0,
                jours_essai: JOURS_ESSAI,
                poste: empreinte,
                titulaire: String::new(),
                numero: String::new(),
                emise_le: String::new(),
                expire_le: String::new(),
                perpetuelle: false,
                sous_licence: true,
            })
        }
    };

    let local = poste(connexion)?;
    let commun = EtatLicence {
        autorise: true,
        statut: STATUT_SOUS_LICENCE.to_string(),
        message: String::new(),
        jours_restants: -1,
        jours_essai: JOURS_ESSAI,
        poste: empreinte,
        titulaire: charge.titulaire.clone(),
        numero: charge.numero.clone(),
        emise_le: charge.emise_le.clone(),
        expire_le: charge.expire_le.clone().unwrap_or_default(),
        perpetuelle: charge.perpetuelle(),
        sous_licence: true,
    };

    if let Some(poste_impose) = charge.poste.as_deref() {
        if !licence::meme_poste(poste_impose, &local) {
            return Ok(EtatLicence {
                autorise: false,
                statut: STATUT_AUTRE_POSTE.to_string(),
                message: format!(
                    "Licence emise pour le poste {poste_impose}, pas pour celui-ci."
                ),
                jours_restants: 0,
                ..commun
            });
        }
    }

    let Some(fin) = charge.expire_le.as_deref() else {
        return Ok(EtatLicence {
            message: format!("Licence perpetuelle au nom de {}.", charge.titulaire),
            jours_restants: -1,
            ..commun
        });
    };

    let restants = jours_restants(maintenant, fin);

    if restants < 0 {
        return Ok(EtatLicence {
            autorise: false,
            statut: STATUT_EXPIREE.to_string(),
            message: format!("Licence expiree le {fin}. Demande une licence a jour."),
            jours_restants: 0,
            ..commun
        });
    }

    Ok(EtatLicence {
        message: format!(
            "Licence au nom de {} : {restants} jour(s) de validite.",
            charge.titulaire
        ),
        jours_restants: restants,
        ..commun
    })
}

/// Une licence vaut jusqu a la fin de son dernier jour : le jour d echeance
/// lui-meme reste ouvert.
fn jours_restants(maintenant: &str, fin: &str) -> i64 {
    horodatage::jours_entre(maintenant, fin).unwrap_or(-1)
}

/// Jour de reference de l essai : le plus avance entre l horloge du poste et
/// le dernier jour deja vu. Une horloge reculee ne rallonge rien.
fn jour_de_reference(connexion: &Connection, maintenant: &str) -> Resultat<String> {
    let vu = parametres::lire_ou(connexion, CLE_ESSAI_DERNIER_JOUR, "")?;
    let aujourd_hui = maintenant.chars().take(10).collect::<String>();

    let retenu = match horodatage::jours_entre(&vu, &aujourd_hui) {
        Some(ecart) if ecart < 0 => vu,
        _ => aujourd_hui,
    };

    parametres::ecrire(connexion, CLE_ESSAI_DERNIER_JOUR, &retenu)?;

    Ok(retenu)
}

fn debut_essai(connexion: &Connection, maintenant: &str) -> Resultat<String> {
    let enregistre = parametres::lire_ou(connexion, CLE_ESSAI_DEBUT, "")?;

    if horodatage::jours_depuis_epoque(&enregistre).is_some() {
        return Ok(enregistre);
    }

    let debut = maintenant.chars().take(10).collect::<String>();

    parametres::ecrire(connexion, CLE_ESSAI_DEBUT, &debut)?;

    Ok(debut)
}

/// Empreinte du poste, tiree au hasard au premier demarrage et conservee avec
/// les parametres. Aucun materiel n est interroge : une carte remplacee ou un
/// disque clone ne doit pas invalider la licence du commerce.
fn poste(connexion: &Connection) -> Resultat<String> {
    let enregistree = parametres::lire_ou(connexion, CLE_POSTE, "")?;

    if !enregistree.trim().is_empty() {
        return Ok(enregistree);
    }

    let mut graine = [0u8; OCTETS_EMPREINTE];

    getrandom::getrandom(&mut graine).map_err(|erreur| {
        ErreurApplication::Conflit(format!("tirage de l empreinte du poste impossible : {erreur}"))
    })?;

    let empreinte = licence::hexadecimal(&graine).to_uppercase();

    parametres::ecrire(connexion, CLE_POSTE, &empreinte)?;

    Ok(empreinte)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::depot::migrations;

    fn base() -> Connection {
        let connexion = Connection::open_in_memory().expect("base en memoire");

        migrations::appliquer(&connexion).expect("migrations");

        connexion
    }

    /// Licence de test : la paire de cles de production n est pas dans le
    /// depot, donc ces essais signent avec leur propre paire et verifient par
    /// `licence::verifier_avec`.
    #[test]
    fn une_base_neuve_ouvre_un_essai_de_sept_jours() {
        let connexion = base();
        let etat = evaluer(&connexion).unwrap();

        assert!(etat.autorise);
        assert_eq!(etat.statut, STATUT_ESSAI);
        assert_eq!(etat.jours_restants, JOURS_ESSAI);
        assert!(!etat.poste.is_empty());
        assert!(exiger_active(&connexion).is_ok());
    }

    #[test]
    fn l_essai_plafonne_les_creations_et_ferme_l_export() {
        let connexion = base();

        // Sous le plafond : rien ne gene.
        assert!(exiger_quota_essai(&connexion, "produits", 5, ESSAI_PRODUITS).is_ok());
        assert!(exiger_quota_essai(&connexion, "caissier(s)", 0, ESSAI_CAISSIERS).is_ok());

        // Au plafond, et au dela : refus.
        let incident = exiger_quota_essai(&connexion, "produits", ESSAI_PRODUITS, ESSAI_PRODUITS)
            .unwrap_err()
            .to_string();

        assert!(incident.contains("version d essai"));
        assert!(incident.contains("6 produits"));
        assert!(exiger_quota_essai(&connexion, "rayon(s)", 7, ESSAI_RAYONS).is_err());
        assert!(exiger_quota_essai(&connexion, "caissier(s)", 1, ESSAI_CAISSIERS).is_err());

        // L export et l import sont fermes tant qu aucune licence n est posee.
        assert!(!sous_licence(&connexion).unwrap());
        assert!(exiger_sous_licence(&connexion, "l export").is_err());
    }

    #[test]
    fn une_licence_illisible_ne_leve_aucun_plafond() {
        let connexion = base();

        parametres::ecrire(&connexion, CLE_LICENCE, "licence-illisible").unwrap();

        let etat = evaluer(&connexion).unwrap();

        // Une licence est bien posee, mais elle n ouvre pas la caisse : les
        // plafonds tiennent, et l export reste ferme.
        assert!(etat.sous_licence);
        assert!(!etat.autorise);
        assert!(!sous_licence(&connexion).unwrap());
        assert!(exiger_quota_essai(&connexion, "produits", 99, ESSAI_PRODUITS).is_err());
        assert!(exiger_sous_licence(&connexion, "l export").is_err());
    }

    #[test]
    fn l_empreinte_du_poste_ne_change_jamais() {
        let connexion = base();
        let premiere = evaluer(&connexion).unwrap().poste;
        let seconde = evaluer(&connexion).unwrap().poste;

        assert_eq!(premiere, seconde);
        assert_eq!(premiere.len(), 19); // 16 caracteres, 3 traits
    }

    #[test]
    fn l_essai_se_ferme_au_bout_de_sept_jours() {
        let connexion = base();

        evaluer(&connexion).unwrap();

        let debut = parametres::lire_ou(&connexion, CLE_ESSAI_DEBUT, "").unwrap();
        let jours = horodatage::jours_depuis_epoque(&debut).unwrap();

        // Sixieme jour : encore ouvert. Septieme : ferme.
        parametres::ecrire(
            &connexion,
            CLE_ESSAI_DERNIER_JOUR,
            &horodatage::date_depuis_epoque(jours + JOURS_ESSAI - 1),
        )
        .unwrap();

        let encore = evaluer(&connexion).unwrap();

        assert!(encore.autorise);
        assert_eq!(encore.jours_restants, 1);

        parametres::ecrire(
            &connexion,
            CLE_ESSAI_DERNIER_JOUR,
            &horodatage::date_depuis_epoque(jours + JOURS_ESSAI),
        )
        .unwrap();

        let termine = evaluer(&connexion).unwrap();

        assert!(!termine.autorise);
        assert_eq!(termine.statut, STATUT_ESSAI_TERMINE);
        assert!(exiger_active(&connexion).is_err());
    }

    #[test]
    fn reculer_l_horloge_ne_rend_pas_un_jour_d_essai() {
        let connexion = base();

        evaluer(&connexion).unwrap();

        let debut = parametres::lire_ou(&connexion, CLE_ESSAI_DEBUT, "").unwrap();
        let jours = horodatage::jours_depuis_epoque(&debut).unwrap();

        parametres::ecrire(
            &connexion,
            CLE_ESSAI_DERNIER_JOUR,
            &horodatage::date_depuis_epoque(jours + 30),
        )
        .unwrap();

        // L horloge du poste revient en arriere : le jour retenu reste le plus
        // avance jamais vu.
        let retenu = jour_de_reference(&connexion, "2000-01-01 08:00:00").unwrap();

        assert_eq!(retenu, horodatage::date_depuis_epoque(jours + 30));
        assert!(!evaluer(&connexion).unwrap().autorise);
    }

    #[test]
    fn une_licence_illisible_ferme_la_caisse() {
        let connexion = base();

        parametres::ecrire(&connexion, CLE_LICENCE, "LCOS1.nimportequoi.signature").unwrap();

        let etat = evaluer(&connexion).unwrap();

        assert!(!etat.autorise);
        assert_eq!(etat.statut, STATUT_REFUSEE);
        assert!(etat.sous_licence);
    }

    #[test]
    fn compte_les_jours_de_validite_restants() {
        assert_eq!(jours_restants("2026-10-24 18:00:00", "2026-10-31"), 7);
        assert_eq!(jours_restants("2026-10-31 18:00:00", "2026-10-31"), 0);
        assert_eq!(jours_restants("2026-11-01 08:00:00", "2026-10-31"), -1);
    }
}
