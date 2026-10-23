/*
 * Jetons d acces en memoire. Le front ne porte aucun droit : il detient un
 * jeton opaque emis apres verification du code secret, et chaque commande
 * sensible le fait valider ici. Rien n est ecrit sur disque : fermer LCOS
 * revoque tout.
 */
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::domaine::secret;
use crate::erreur::{ErreurApplication, Resultat};

pub const ROLE_GERANT: &str = "gerant";
pub const ROLE_CAISSIER: &str = "caissier";

const DUREE_INACTIVITE: Duration = Duration::from_secs(15 * 60);
const OCTETS_JETON: usize = 24;
const JETONS_MAXIMAUX: usize = 16;

#[derive(Debug, Clone)]
pub struct Acces {
    pub compte: i64,
}

struct Entree {
    compte: i64,
    role: String,
    dernier_usage: Instant,
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

    pub fn emettre(&self, compte: i64, role: &str) -> Resultat<String> {
        let mut octets = [0u8; OCTETS_JETON];
        getrandom::getrandom(&mut octets)
            .map_err(|_| ErreurApplication::Validation("alea systeme indisponible".to_string()))?;

        let jeton = secret::vers_hexadecimal(&octets);

        let mut entrees = self
            .entrees
            .lock()
            .map_err(|_| ErreurApplication::VerrouCorrompu)?;

        purger(&mut entrees);

        if entrees.len() >= JETONS_MAXIMAUX {
            entrees.clear();
        }

        entrees.insert(
            jeton.clone(),
            Entree {
                compte,
                role: role.to_string(),
                dernier_usage: Instant::now(),
            },
        );

        Ok(jeton)
    }

    /// Valide le jeton pour le role attendu et prolonge sa duree de vie.
    pub fn exiger(&self, jeton: &str, role_attendu: &str) -> Resultat<Acces> {
        let mut entrees = self
            .entrees
            .lock()
            .map_err(|_| ErreurApplication::VerrouCorrompu)?;

        purger(&mut entrees);

        let entree = entrees
            .get_mut(jeton)
            .ok_or_else(|| ErreurApplication::AccesRefuse("session expiree".to_string()))?;

        if entree.role != role_attendu {
            return Err(ErreurApplication::AccesRefuse(format!(
                "role {role_attendu} requis"
            )));
        }

        entree.dernier_usage = Instant::now();

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

fn purger(entrees: &mut HashMap<String, Entree>) {
    let maintenant = Instant::now();

    entrees.retain(|_, entree| {
        maintenant.duration_since(entree.dernier_usage) < DUREE_INACTIVITE
    });
}
