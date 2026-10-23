/*
 * Codes secrets du gerant et des caissiers.
 * Jamais stockes en clair : PBKDF2-HMAC-SHA256, sel aleatoire de 16 octets par
 * compte, comparaison a temps constant. Le nombre d iterations est conserve
 * avec l empreinte pour pouvoir le relever sans invalider les codes existants.
 */
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::erreur::{ErreurApplication, Resultat};

type HmacSha256 = Hmac<Sha256>;

pub const LONGUEUR_MINIMALE: usize = 4;
pub const LONGUEUR_MAXIMALE: usize = 8;
pub const ITERATIONS: u32 = 300_000;

const OCTETS_SEL: usize = 16;
const OCTETS_EMPREINTE: usize = 32;

#[derive(Debug, Clone)]
pub struct Empreinte {
    pub empreinte: String,
    pub sel: String,
    pub iterations: u32,
}

/// Un code secret de caisse : chiffres uniquement, frappe au pave numerique.
pub fn valider(code: &str) -> Resultat<&str> {
    if code.len() < LONGUEUR_MINIMALE || code.len() > LONGUEUR_MAXIMALE {
        return Err(ErreurApplication::Validation(format!(
            "code secret : de {LONGUEUR_MINIMALE} a {LONGUEUR_MAXIMALE} chiffres"
        )));
    }

    if !code.chars().all(|caractere| caractere.is_ascii_digit()) {
        return Err(ErreurApplication::Validation(
            "code secret : chiffres uniquement".to_string(),
        ));
    }

    Ok(code)
}

pub fn sceller(code: &str) -> Resultat<Empreinte> {
    sceller_avec(code, ITERATIONS)
}

fn sceller_avec(code: &str, iterations: u32) -> Resultat<Empreinte> {
    valider(code)?;

    let mut sel = [0u8; OCTETS_SEL];
    getrandom::getrandom(&mut sel)
        .map_err(|_| ErreurApplication::Validation("alea systeme indisponible".to_string()))?;

    let empreinte = deriver(code.as_bytes(), &sel, iterations);

    Ok(Empreinte {
        empreinte: vers_hexadecimal(&empreinte),
        sel: vers_hexadecimal(&sel),
        iterations,
    })
}

pub fn verifier(code: &str, reference: &Empreinte) -> bool {
    let sel = match depuis_hexadecimal(&reference.sel) {
        Some(octets) => octets,
        None => return false,
    };

    let attendue = match depuis_hexadecimal(&reference.empreinte) {
        Some(octets) => octets,
        None => return false,
    };

    if attendue.len() != OCTETS_EMPREINTE || reference.iterations == 0 {
        return false;
    }

    let calculee = deriver(code.as_bytes(), &sel, reference.iterations);

    comparer_temps_constant(&calculee, &attendue)
}

fn deriver(code: &[u8], sel: &[u8], iterations: u32) -> [u8; OCTETS_EMPREINTE] {
    // Etat HMAC deja cle : clone a chaque tour plutot que de rederiver la cle.
    let socle = HmacSha256::new_from_slice(code).expect("hmac accepte toute longueur de cle");

    let mut bloc = {
        let mut mac = socle.clone();
        mac.update(sel);
        mac.update(&1u32.to_be_bytes());
        mac.finalize().into_bytes()
    };

    let mut resultat = bloc;

    for _ in 1..iterations {
        let mut mac = socle.clone();
        mac.update(&bloc);
        bloc = mac.finalize().into_bytes();

        for (octet, apport) in resultat.iter_mut().zip(bloc.iter()) {
            *octet ^= apport;
        }
    }

    resultat.into()
}

fn comparer_temps_constant(gauche: &[u8], droite: &[u8]) -> bool {
    if gauche.len() != droite.len() {
        return false;
    }

    let mut ecart = 0u8;

    for (a, b) in gauche.iter().zip(droite.iter()) {
        ecart |= a ^ b;
    }

    ecart == 0
}

pub fn vers_hexadecimal(octets: &[u8]) -> String {
    let mut texte = String::with_capacity(octets.len() * 2);

    for octet in octets {
        texte.push(chiffre_hexadecimal(octet >> 4));
        texte.push(chiffre_hexadecimal(octet & 0x0f));
    }

    texte
}

fn chiffre_hexadecimal(quartet: u8) -> char {
    match quartet {
        0..=9 => (b'0' + quartet) as char,
        _ => (b'a' + quartet - 10) as char,
    }
}

fn depuis_hexadecimal(texte: &str) -> Option<Vec<u8>> {
    if texte.len() % 2 != 0 {
        return None;
    }

    let octets = texte.as_bytes();
    let mut resultat = Vec::with_capacity(octets.len() / 2);

    for paire in octets.chunks(2) {
        let haut = quartet(paire[0])?;
        let bas = quartet(paire[1])?;

        resultat.push((haut << 4) | bas);
    }

    Some(resultat)
}

fn quartet(caractere: u8) -> Option<u8> {
    match caractere {
        b'0'..=b'9' => Some(caractere - b'0'),
        b'a'..=b'f' => Some(caractere - b'a' + 10),
        b'A'..=b'F' => Some(caractere - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respecte_les_vecteurs_pbkdf2_sha256() {
        // Vecteurs publics de reference PBKDF2-HMAC-SHA256, dkLen = 32.
        assert_eq!(
            vers_hexadecimal(&deriver(b"password", b"salt", 1)),
            "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"
        );

        assert_eq!(
            vers_hexadecimal(&deriver(b"password", b"salt", 2)),
            "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43"
        );
    }

    #[test]
    fn valide_la_forme_du_code() {
        assert!(valider("1234").is_ok());
        assert!(valider("12345678").is_ok());
        assert!(valider("123").is_err());
        assert!(valider("123456789").is_err());
        assert!(valider("12a4").is_err());
        assert!(valider("").is_err());
    }

    #[test]
    fn reconnait_le_bon_code() {
        let reference = Empreinte {
            sel: vers_hexadecimal(b"selselselselsels"),
            empreinte: vers_hexadecimal(&deriver(b"4821", b"selselselselsels", 1000)),
            iterations: 1000,
        };

        assert!(verifier("4821", &reference));
        assert!(!verifier("4822", &reference));
        assert!(!verifier("", &reference));
    }

    #[test]
    fn deux_scellements_du_meme_code_divergent() {
        let premiere = sceller_avec("1234", 1000).unwrap();
        let seconde = sceller_avec("1234", 1000).unwrap();

        assert_ne!(premiere.sel, seconde.sel);
        assert_ne!(premiere.empreinte, seconde.empreinte);
        assert!(verifier("1234", &premiere));
        assert!(verifier("1234", &seconde));
    }

    #[test]
    fn rejette_une_empreinte_corrompue() {
        let mut reference = sceller_avec("1234", 1000).unwrap();
        reference.empreinte = "zz".to_string();

        assert!(!verifier("1234", &reference));
    }
}
