/*
 * Freinage des essais de code secret. Un code de caisse fait 8 chiffres au
 * maximum : sans freinage, un essai en boucle le trouve. Apres plusieurs
 * echecs, la cible est bloquee quelques instants.
 */
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::erreur::{ErreurApplication, Resultat};

const ECHECS_TOLERES: u32 = 5;
const BLOCAGE: Duration = Duration::from_secs(60);
const OUBLI: Duration = Duration::from_secs(600);

struct Etat {
    echecs: u32,
    dernier_echec: Instant,
}

pub struct Limiteur {
    etats: Mutex<HashMap<String, Etat>>,
}

impl Limiteur {
    pub fn nouveau() -> Self {
        Self {
            etats: Mutex::new(HashMap::new()),
        }
    }

    /// A appeler avant toute verification de code.
    pub fn verifier(&self, cible: &str) -> Resultat<()> {
        let mut etats = self
            .etats
            .lock()
            .map_err(|_| ErreurApplication::VerrouCorrompu)?;

        oublier(&mut etats);

        if let Some(etat) = etats.get(cible) {
            let attente = etat.dernier_echec.elapsed();

            if etat.echecs >= ECHECS_TOLERES && attente < BLOCAGE {
                let reste = (BLOCAGE - attente).as_secs() + 1;

                return Err(ErreurApplication::AccesRefuse(format!(
                    "trop d essais : reessayer dans {reste} secondes"
                )));
            }
        }

        Ok(())
    }

    pub fn echec(&self, cible: &str) {
        if let Ok(mut etats) = self.etats.lock() {
            let etat = etats.entry(cible.to_string()).or_insert(Etat {
                echecs: 0,
                dernier_echec: Instant::now(),
            });

            // Un nouveau cycle apres la fin du blocage repart de zero.
            if etat.echecs >= ECHECS_TOLERES && etat.dernier_echec.elapsed() >= BLOCAGE {
                etat.echecs = 0;
            }

            etat.echecs += 1;
            etat.dernier_echec = Instant::now();
        }
    }

    pub fn reussite(&self, cible: &str) {
        if let Ok(mut etats) = self.etats.lock() {
            etats.remove(cible);
        }
    }
}

fn oublier(etats: &mut HashMap<String, Etat>) {
    etats.retain(|_, etat| etat.dernier_echec.elapsed() < OUBLI);
}
