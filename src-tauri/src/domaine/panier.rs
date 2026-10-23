/*
 * Arithmetique de la vente. Volontairement pure : aucune dependance a la base,
 * les prix arrivent deja resolus depuis le catalogue.
 */
use crate::erreur::{ErreurApplication, Resultat};

use super::monnaie;

pub const QUANTITE_MAXIMALE: i64 = 9_999;
pub const LIGNES_MAXIMALES: usize = 500;

#[derive(Debug, Clone, Copy)]
pub struct LigneChiffree {
    pub prix_centimes: i64,
    pub quantite: i64,
}

pub fn valider_quantite(quantite: i64) -> Resultat<i64> {
    if quantite <= 0 {
        return Err(ErreurApplication::Validation(
            "quantite nulle ou negative refusee".to_string(),
        ));
    }

    if quantite > QUANTITE_MAXIMALE {
        return Err(ErreurApplication::Validation(format!(
            "quantite hors limite : maximum {QUANTITE_MAXIMALE}"
        )));
    }

    Ok(quantite)
}

pub fn total_ligne(ligne: LigneChiffree) -> Resultat<i64> {
    let total = ligne
        .prix_centimes
        .checked_mul(ligne.quantite)
        .ok_or_else(|| ErreurApplication::Validation("total de ligne hors limite".to_string()))?;

    monnaie::valider(total)
}

pub fn total_panier(lignes: &[LigneChiffree]) -> Resultat<i64> {
    if lignes.is_empty() {
        return Err(ErreurApplication::Validation(
            "panier vide : rien a encaisser".to_string(),
        ));
    }

    if lignes.len() > LIGNES_MAXIMALES {
        return Err(ErreurApplication::Validation(format!(
            "panier hors limite : maximum {LIGNES_MAXIMALES} lignes"
        )));
    }

    let mut total = 0i64;

    for ligne in lignes {
        valider_quantite(ligne.quantite)?;

        total = total
            .checked_add(total_ligne(*ligne)?)
            .ok_or_else(|| ErreurApplication::Validation("total hors limite".to_string()))?;
    }

    monnaie::valider(total)
}

pub fn nombre_articles(lignes: &[LigneChiffree]) -> i64 {
    lignes.iter().map(|ligne| ligne.quantite).sum()
}

/// Reliquat a remettre au client. Refuse un encaissement insuffisant.
pub fn calculer_rendu(total_centimes: i64, recu_centimes: i64) -> Resultat<i64> {
    monnaie::valider(total_centimes)?;
    monnaie::valider(recu_centimes)?;

    if recu_centimes < total_centimes {
        return Err(ErreurApplication::Validation(format!(
            "somme recue insuffisante : il manque {}",
            monnaie::formater(total_centimes - recu_centimes)
        )));
    }

    Ok(recu_centimes - total_centimes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ligne(prix: i64, quantite: i64) -> LigneChiffree {
        LigneChiffree {
            prix_centimes: prix,
            quantite,
        }
    }

    #[test]
    fn somme_les_lignes() {
        let lignes = [ligne(150, 3), ligne(99, 1), ligne(1000, 2)];

        assert_eq!(total_panier(&lignes).unwrap(), 450 + 99 + 2000);
        assert_eq!(nombre_articles(&lignes), 6);
    }

    #[test]
    fn refuse_un_panier_vide() {
        assert!(total_panier(&[]).is_err());
    }

    #[test]
    fn refuse_une_quantite_invalide() {
        assert!(total_panier(&[ligne(100, 0)]).is_err());
        assert!(total_panier(&[ligne(100, QUANTITE_MAXIMALE + 1)]).is_err());
    }

    #[test]
    fn rend_la_difference() {
        assert_eq!(calculer_rendu(1550, 2000).unwrap(), 450);
        assert_eq!(calculer_rendu(1550, 1550).unwrap(), 0);
    }

    #[test]
    fn refuse_un_paiement_insuffisant() {
        let incident = calculer_rendu(2000, 1500).unwrap_err();

        assert!(incident.to_string().contains("5,00"));
    }
}
