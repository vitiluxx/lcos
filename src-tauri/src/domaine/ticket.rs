/*
 * Mise en page du ticket et du rapport Z en texte brut, sur une largeur fixe
 * en colonnes. L imprimante thermique 58 mm integree rend 32 colonnes ; la
 * meme sortie est affichee a l ecran et envoyee au papier, donc ce qui est
 * verifie ici est exactement ce qui sort.
 */
use serde::Serialize;

use super::monnaie;

pub const COLONNES_58MM: usize = 32;
const COLONNES_MINIMALES: usize = 24;
const COLONNES_MAXIMALES: usize = 80;

#[derive(Debug, Serialize)]
pub struct LigneTicket {
    pub libelle: String,
    pub quantite: i64,
    pub prix_centimes: i64,
    pub total_centimes: i64,
}

#[derive(Debug, Serialize)]
pub struct DonneesTicket {
    pub nom_commerce: String,
    pub entete: String,
    pub pied: String,
    pub devise: String,
    pub numero_ticket: i64,
    pub horodatage: String,
    pub caissier: String,
    pub lignes: Vec<LigneTicket>,
    pub nombre_articles: i64,
    pub total_centimes: i64,
    pub recu_centimes: i64,
    pub rendu_centimes: i64,
}

#[derive(Debug, Serialize)]
pub struct TotalNomme {
    pub libelle: String,
    pub quantite: i64,
    pub total_centimes: i64,
}

#[derive(Debug, Serialize)]
pub struct VenteResumee {
    pub numero_ticket: i64,
    pub horodatage: String,
    pub caissier: String,
    pub nombre_articles: i64,
    pub total_centimes: i64,
}

#[derive(Debug, Serialize)]
pub struct DonneesRapportZ {
    pub numero: i64,
    pub nom_commerce: String,
    pub devise: String,
    pub genere_le: String,
    pub debut_le: String,
    pub fin_le: String,
    pub nombre_ventes: i64,
    pub nombre_articles: i64,
    pub total_centimes: i64,
    pub premier_ticket: i64,
    pub dernier_ticket: i64,
    pub par_caissier: Vec<TotalNomme>,
    pub par_rayon: Vec<TotalNomme>,
    pub ventes: Vec<VenteResumee>,
    pub grand_total_centimes: i64,
    pub grand_total_articles: i64,
    pub grand_total_ventes: i64,
    pub remises_a_zero: i64,
}

pub fn borner_colonnes(colonnes: i64) -> usize {
    let valeur = colonnes.clamp(COLONNES_MINIMALES as i64, COLONNES_MAXIMALES as i64);

    valeur as usize
}

pub fn rendre_ticket(donnees: &DonneesTicket, colonnes: usize) -> String {
    let mut lignes: Vec<String> = Vec::new();

    lignes.push(centrer(&donnees.nom_commerce.to_uppercase(), colonnes));

    if !donnees.entete.trim().is_empty() {
        lignes.push(centrer(donnees.entete.trim(), colonnes));
    }

    lignes.push(separateur(colonnes));
    lignes.push(paire(
        &format!("Ticket no {}", donnees.numero_ticket),
        &donnees.horodatage,
        colonnes,
    ));
    lignes.push(tronquer(
        &format!("Caissier : {}", donnees.caissier),
        colonnes,
    ));
    lignes.push(separateur(colonnes));

    for ligne in &donnees.lignes {
        lignes.push(tronquer(&ligne.libelle, colonnes));
        lignes.push(paire(
            &format!(
                "  {} x {}",
                ligne.quantite,
                monnaie::formater(ligne.prix_centimes)
            ),
            &monnaie::formater(ligne.total_centimes),
            colonnes,
        ));
    }

    lignes.push(separateur(colonnes));
    lignes.push(paire(
        &format!("TOTAL {}", donnees.devise),
        &monnaie::formater(donnees.total_centimes),
        colonnes,
    ));
    lignes.push(paire(
        "Recu",
        &monnaie::formater(donnees.recu_centimes),
        colonnes,
    ));
    lignes.push(paire(
        "Rendu",
        &monnaie::formater(donnees.rendu_centimes),
        colonnes,
    ));
    lignes.push(paire(
        "Articles",
        &donnees.nombre_articles.to_string(),
        colonnes,
    ));

    if !donnees.pied.trim().is_empty() {
        lignes.push(separateur(colonnes));
        lignes.push(centrer(donnees.pied.trim(), colonnes));
    }

    lignes.join("\n")
}

pub fn rendre_rapport_z(donnees: &DonneesRapportZ, colonnes: usize) -> String {
    let mut lignes: Vec<String> = Vec::new();

    lignes.push(centrer(&donnees.nom_commerce.to_uppercase(), colonnes));
    lignes.push(centrer(&format!("RAPPORT Z no {}", donnees.numero), colonnes));
    lignes.push(centrer(&donnees.genere_le, colonnes));
    lignes.push(separateur(colonnes));

    lignes.push(paire("Premiere vente", &donnees.debut_le, colonnes));
    lignes.push(paire("Derniere vente", &donnees.fin_le, colonnes));
    lignes.push(paire(
        "Tickets",
        &format!("{} a {}", donnees.premier_ticket, donnees.dernier_ticket),
        colonnes,
    ));
    lignes.push(separateur(colonnes));
    lignes.push(centrer("DETAIL DES VENTES", colonnes));

    if donnees.ventes.is_empty() {
        lignes.push(centrer("aucune vente", colonnes));
    }

    for vente in &donnees.ventes {
        lignes.push(paire(
            &format!("no {} {}", vente.numero_ticket, vente.horodatage),
            &monnaie::formater(vente.total_centimes),
            colonnes,
        ));
    }

    lignes.push(separateur(colonnes));
    lignes.push(centrer("PAR CAISSIER", colonnes));

    for total in &donnees.par_caissier {
        lignes.push(paire(
            &format!("{} ({})", total.libelle, total.quantite),
            &monnaie::formater(total.total_centimes),
            colonnes,
        ));
    }

    lignes.push(separateur(colonnes));
    lignes.push(centrer("PAR RAYON", colonnes));

    for total in &donnees.par_rayon {
        lignes.push(paire(
            &format!("{} ({})", total.libelle, total.quantite),
            &monnaie::formater(total.total_centimes),
            colonnes,
        ));
    }

    lignes.push(separateur(colonnes));
    lignes.push(paire("Ventes", &donnees.nombre_ventes.to_string(), colonnes));
    lignes.push(paire(
        "Articles",
        &donnees.nombre_articles.to_string(),
        colonnes,
    ));
    lignes.push(paire(
        &format!("TOTAL {}", donnees.devise),
        &monnaie::formater(donnees.total_centimes),
        colonnes,
    ));
    lignes.push(separateur(colonnes));
    lignes.push(centrer("GRANDS TOTAUX", colonnes));
    lignes.push(paire(
        "Cumul ventes",
        &donnees.grand_total_ventes.to_string(),
        colonnes,
    ));
    lignes.push(paire(
        "Cumul articles",
        &donnees.grand_total_articles.to_string(),
        colonnes,
    ));
    lignes.push(paire(
        "Cumul encaisse",
        &monnaie::formater(donnees.grand_total_centimes),
        colonnes,
    ));
    lignes.push(paire(
        "Remises a zero",
        &donnees.remises_a_zero.to_string(),
        colonnes,
    ));

    lignes.join("\n")
}

fn separateur(colonnes: usize) -> String {
    "-".repeat(colonnes)
}

fn centrer(texte: &str, colonnes: usize) -> String {
    let coupe = tronquer(texte, colonnes);
    let longueur = coupe.chars().count();

    if longueur >= colonnes {
        return coupe;
    }

    " ".repeat((colonnes - longueur) / 2) + &coupe
}

/// Libelle a gauche, valeur a droite, une seule ligne garantie.
fn paire(gauche: &str, droite: &str, colonnes: usize) -> String {
    let valeur = tronquer(droite, colonnes);
    let place_valeur = valeur.chars().count();
    let place_libelle = colonnes.saturating_sub(place_valeur + 1);
    let libelle = tronquer(gauche, place_libelle);
    let remplissage = colonnes - libelle.chars().count() - place_valeur;

    libelle + &" ".repeat(remplissage) + &valeur
}

fn tronquer(texte: &str, colonnes: usize) -> String {
    texte.chars().take(colonnes).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn donnees_minimales() -> DonneesTicket {
        DonneesTicket {
            nom_commerce: "Mon commerce".to_string(),
            entete: "Merci de votre visite".to_string(),
            pied: "A bientot".to_string(),
            devise: "EUR".to_string(),
            numero_ticket: 42,
            horodatage: "24/10/2026 01:12".to_string(),
            caissier: "Ali".to_string(),
            lignes: vec![
                LigneTicket {
                    libelle: "Coca 33cl".to_string(),
                    quantite: 2,
                    prix_centimes: 150,
                    total_centimes: 300,
                },
                LigneTicket {
                    libelle: "Pain".to_string(),
                    quantite: 1,
                    prix_centimes: 90,
                    total_centimes: 90,
                },
            ],
            nombre_articles: 3,
            total_centimes: 390,
            recu_centimes: 500,
            rendu_centimes: 110,
        }
    }

    #[test]
    fn aucune_ligne_ne_depasse_la_largeur() {
        let rendu = rendre_ticket(&donnees_minimales(), COLONNES_58MM);

        for ligne in rendu.lines() {
            assert!(
                ligne.chars().count() <= COLONNES_58MM,
                "ligne trop large : {ligne}"
            );
        }
    }

    #[test]
    fn porte_les_montants_cles() {
        let rendu = rendre_ticket(&donnees_minimales(), COLONNES_58MM);

        assert!(rendu.contains("Ticket no 42"));
        assert!(rendu.contains("3,90"));
        assert!(rendu.contains("Rendu"));
        assert!(rendu.contains("1,10"));
        assert!(rendu.contains("MON COMMERCE"));
    }

    #[test]
    fn aligne_la_valeur_a_droite() {
        assert_eq!(paire("TOTAL", "3,90", 16), "TOTAL       3,90");
    }

    #[test]
    fn tronque_un_libelle_trop_long() {
        let ligne = paire("libelle beaucoup trop long pour la ligne", "9,99", 16);

        assert_eq!(ligne.chars().count(), 16);
        assert!(ligne.ends_with("9,99"));
    }

    #[test]
    fn borne_la_largeur_demandee() {
        assert_eq!(borner_colonnes(32), 32);
        assert_eq!(borner_colonnes(0), COLONNES_MINIMALES);
        assert_eq!(borner_colonnes(500), COLONNES_MAXIMALES);
    }
}
