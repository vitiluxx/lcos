/*
 * Jetons d acces en memoire. Le front ne porte aucun droit : il detient un
 * jeton opaque emis apres verification du code secret, et chaque commande
 * sensible le fait valider ici. Rien n est ecrit sur disque : fermer LCOS
 * revoque tout.
 *
 * Aucune peremption par l horloge. Un minuteur d inactivite coupait le
 * parametrage au milieu d un travail long : programmer cinquante produits en
 * lisant les etiquettes depasse le quart d heure entre deux clics, et le
 * gerant se retrouvait devant des erreurs opaques, ecran encore deverrouille,
 * sans autre issue que verrouiller puis deverrouiller.
 *
 * Le verrou suit donc l ecran et non la montre : quitter le parametrage pour
 * la caisse ou la session revoque le jeton (`vueParametrage.sortir`), et un
 * seul jeton vit a la fois, le nouveau chassant l ancien.
 */
use std::collections::HashMap;
use std::sync::Mutex;

use crate::domaine::secret;
use crate::erreur::{ErreurApplication, Resultat};

pub const ROLE_GERANT: &str = "gerant";
pub const ROLE_CAISSIER: &str = "caissier";

const OCTETS_JETON: usize = 24;

#[derive(Debug, Clone)]
pub struct Acces {
    pub compte: i64,
}

struct Entree {
    compte: i64,
    role: String,
}

pub struct RegistreJetons {
    entrees: Mutex<HashMap<String, Entree>>,
}

impl RegistreJetons {
    pub fn nouveau() -> Self {
        Self {
            entrees: Mutex::new(HashMap::new()),
        }
    }

    /// Emet un jeton et revoque tous les precedents : la caisse n a qu un
    /// gerant, une seule fenetre, donc un seul acces ouvert. Un deuxieme
    /// deverrouillage rend le premier jeton inutilisable au lieu de le
    /// laisser trainer en memoire.
    pub fn emettre(&self, compte: i64, role: &str) -> Resultat<String> {
        let mut octets = [0u8; OCTETS_JETON];
        getrandom::getrandom(&mut octets)
            .map_err(|_| ErreurApplication::Validation("alea systeme indisponible".to_string()))?;

        let jeton = secret::vers_hexadecimal(&octets);

        let mut entrees = self
            .entrees
            .lock()
            .map_err(|_| ErreurApplication::VerrouCorrompu)?;

        entrees.clear();
        entrees.insert(
            jeton.clone(),
            Entree {
                compte,
                role: role.to_string(),
            },
        );

        Ok(jeton)
    }

    /// Valide le jeton pour le role attendu.
    pub fn exiger(&self, jeton: &str, role_attendu: &str) -> Resultat<Acces> {
        let entrees = self
            .entrees
            .lock()
            .map_err(|_| ErreurApplication::VerrouCorrompu)?;

        let entree = entrees
            .get(jeton)
            .ok_or_else(|| ErreurApplication::AccesRefuse("parametrage verrouille".to_string()))?;

        if entree.role != role_attendu {
            return Err(ErreurApplication::AccesRefuse(format!(
                "role {role_attendu} requis"
            )));
        }

        Ok(Acces {
            compte: entree.compte,
        })
    }

    pub fn revoquer(&self, jeton: &str) {
        if let Ok(mut entrees) = self.entrees.lock() {
            entrees.remove(jeton);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_jeton_reste_valable_quel_que_soit_le_temps_passe() {
        let registre = RegistreJetons::nouveau();
        let jeton = registre.emettre(1, ROLE_GERANT).unwrap();

        // Aucune horloge dans le registre : rien ne peut perimer le jeton
        // entre deux appels, c est la sortie du parametrage qui le revoque.
        assert_eq!(registre.exiger(&jeton, ROLE_GERANT).unwrap().compte, 1);
        assert_eq!(registre.exiger(&jeton, ROLE_GERANT).unwrap().compte, 1);
    }

    #[test]
    fn un_nouveau_jeton_revoque_le_precedent() {
        let registre = RegistreJetons::nouveau();
        let premier = registre.emettre(1, ROLE_GERANT).unwrap();
        let second = registre.emettre(1, ROLE_GERANT).unwrap();

        assert!(premier != second);
        assert!(registre.exiger(&premier, ROLE_GERANT).is_err());
        assert!(registre.exiger(&second, ROLE_GERANT).is_ok());
    }

    #[test]
    fn la_revocation_ferme_l_acces() {
        let registre = RegistreJetons::nouveau();
        let jeton = registre.emettre(1, ROLE_GERANT).unwrap();

        registre.revoquer(&jeton);

        assert!(registre.exiger(&jeton, ROLE_GERANT).is_err());
    }

    #[test]
    fn refuse_un_role_qui_n_est_pas_celui_du_jeton() {
        let registre = RegistreJetons::nouveau();
        let jeton = registre.emettre(2, ROLE_CAISSIER).unwrap();

        assert!(registre.exiger(&jeton, ROLE_GERANT).is_err());
        assert!(registre.exiger("jeton inconnu", ROLE_CAISSIER).is_err());
    }
}
