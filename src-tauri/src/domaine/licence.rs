/*
 * Licence d utilisation de LCOS.
 *
 * Une licence est un texte signe, verifiable hors ligne et pour toujours :
 *
 *     LCOS1.<charge en base64url>.<signature en base64url>
 *
 * La signature est une Ed25519. Le logiciel ne porte que la cle **publique**,
 * donc il sait verifier une licence mais ne sait pas en fabriquer : seul le
 * detenteur de la cle privee peut en emettre, sur sa propre machine, des mois
 * apres l installation, sans rien changer au logiciel deja livre.
 *
 * Ce module ne touche ni a la base ni au systeme : il ne fait que lire, ecrire
 * et verifier du texte. La politique (essai gratuit, blocage) est ailleurs.
 */
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::erreur::{ErreurApplication, Resultat};

/// Marque de format. Un changement de schema de charge changera ce prefixe,
/// pour qu une vieille version refuse clairement une licence trop recente.
pub const PREFIXE: &str = "LCOS1";

/// Cle publique de verification, en hexadecimal (32 octets). La cle privee
/// correspondante ne figure nulle part dans ce depot : elle reste chez
/// l editeur, hors du code et hors du setup.
pub const CLE_PUBLIQUE_HEX: &str =
    "1e57a32ddb0a432fec9af90b2870dce52d3252a63d1c06c42f1cf00b09399b7b";

const LONGUEUR_TEXTE_MAXIMALE: usize = 4096;
const LONGUEUR_CHAMP_MAXIMALE: usize = 80;

/// Contenu signe d une licence. Tout ce qui est ici est verifie par la
/// signature : modifier un seul caractere invalide la licence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Charge {
    /// Numero d emission, pour retrouver a qui une licence a ete delivree.
    pub numero: String,
    /// Nom du commerce ou du client.
    pub titulaire: String,
    /// Date d emission, "AAAA-MM-JJ".
    pub emise_le: String,
    /// Fin de validite, "AAAA-MM-JJ". Absente : licence perpetuelle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expire_le: Option<String>,
    /// Empreinte du poste autorise. Absente : la licence vaut pour tout poste.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poste: Option<String>,
}

impl Charge {
    pub fn perpetuelle(&self) -> bool {
        self.expire_le.is_none()
    }
}

/// Signe une charge. Reservee a l outil en ligne de commande de l editeur :
/// le logiciel livre n appelle jamais cette fonction, et n embarque pas de
/// cle privee pour le faire.
pub fn signer(charge: &Charge, cle_privee_hex: &str) -> Resultat<String> {
    let octets = octets_de_cle(cle_privee_hex, "cle privee")?;
    let cle = ed25519_dalek::SigningKey::from_bytes(&octets);
    let charge_json = serde_json::to_vec(charge)
        .map_err(|erreur| ErreurApplication::Validation(format!("charge illisible : {erreur}")))?;

    let signature = ed25519_dalek::Signer::sign(&cle, &charge_json);

    Ok(format!(
        "{PREFIXE}.{}.{}",
        base64url::encoder(&charge_json),
        base64url::encoder(&signature.to_bytes())
    ))
}

/// Verifie une licence saisie par le gerant. Les espaces et les retours a la
/// ligne sont ignores : une licence se colle souvent depuis un courriel ou un
/// fichier, ou elle arrive coupee en morceaux.
pub fn verifier(texte: &str) -> Resultat<Charge> {
    verifier_avec(texte, CLE_PUBLIQUE_HEX)
}

pub fn verifier_avec(texte: &str, cle_publique_hex: &str) -> Resultat<Charge> {
    if texte.len() > LONGUEUR_TEXTE_MAXIMALE {
        return Err(ErreurApplication::Validation(
            "licence trop longue : ce n est pas une licence LCOS".to_string(),
        ));
    }

    let compacte: String = texte.chars().filter(|c| !c.is_whitespace()).collect();

    if compacte.is_empty() {
        return Err(ErreurApplication::Validation(
            "aucune licence saisie".to_string(),
        ));
    }

    let tranches: Vec<&str> = compacte.split('.').collect();

    if tranches.len() != 3 {
        return Err(ErreurApplication::Validation(
            "licence mal formee : trois parties separees par un point attendues".to_string(),
        ));
    }

    if tranches[0] != PREFIXE {
        return Err(ErreurApplication::Validation(format!(
            "licence de type {} inconnu : cette version lit les licences {PREFIXE}",
            tranches[0]
        )));
    }

    let charge_json = base64url::decodeur(tranches[1])
        .ok_or_else(|| ErreurApplication::Validation("licence illisible : charge".to_string()))?;
    let signature_octets = base64url::decodeur(tranches[2]).ok_or_else(|| {
        ErreurApplication::Validation("licence illisible : signature".to_string())
    })?;

    let cle = VerifyingKey::from_bytes(&octets_de_cle(cle_publique_hex, "cle publique")?)
        .map_err(|_| ErreurApplication::Validation("cle de verification invalide".to_string()))?;

    let signature = Signature::from_slice(&signature_octets).map_err(|_| {
        ErreurApplication::Validation("signature de licence invalide".to_string())
    })?;

    cle.verify(&charge_json, &signature).map_err(|_| {
        ErreurApplication::AccesRefuse(
            "licence refusee : signature non reconnue. Verifie la saisie, ou demande une licence a l editeur".to_string(),
        )
    })?;

    let charge: Charge = serde_json::from_slice(&charge_json).map_err(|erreur| {
        ErreurApplication::Validation(format!("licence illisible : contenu ({erreur})"))
    })?;

    valider_champs(&charge)?;

    Ok(charge)
}

fn valider_champs(charge: &Charge) -> Resultat<()> {
    for (valeur, libelle) in [
        (charge.numero.as_str(), "numero"),
        (charge.titulaire.as_str(), "titulaire"),
        (charge.emise_le.as_str(), "date d emission"),
    ] {
        if valeur.trim().is_empty() || valeur.len() > LONGUEUR_CHAMP_MAXIMALE {
            return Err(ErreurApplication::Validation(format!(
                "licence illisible : {libelle}"
            )));
        }
    }

    Ok(())
}

fn octets_de_cle(hexadecimal: &str, libelle: &str) -> Resultat<[u8; 32]> {
    let nettoye: String = hexadecimal
        .chars()
        .filter(|caractere| !caractere.is_whitespace())
        .collect();

    if nettoye.len() != 64 {
        return Err(ErreurApplication::Validation(format!(
            "{libelle} : 64 caracteres hexadecimaux attendus"
        )));
    }

    let mut octets = [0u8; 32];

    for (rang, paire) in nettoye.as_bytes().chunks(2).enumerate() {
        let texte = std::str::from_utf8(paire).unwrap_or("zz");

        octets[rang] = u8::from_str_radix(texte, 16).map_err(|_| {
            ErreurApplication::Validation(format!("{libelle} : hexadecimal invalide"))
        })?;
    }

    Ok(octets)
}

pub fn hexadecimal(octets: &[u8]) -> String {
    octets
        .iter()
        .map(|octet| format!("{octet:02x}"))
        .collect::<String>()
}

/// Empreinte de poste telle qu elle est lue a l ecran : quatre groupes de
/// quatre, separes par des traits, pour la dicter au telephone sans erreur.
pub fn formater_poste(brut: &str) -> String {
    let compacte = normaliser_poste(brut);

    compacte
        .as_bytes()
        .chunks(4)
        .map(|morceau| String::from_utf8_lossy(morceau).to_string())
        .collect::<Vec<String>>()
        .join("-")
}

/// Comparaison des empreintes de poste : la casse et les separateurs ne
/// comptent pas, seule la suite de caracteres utiles.
pub fn normaliser_poste(brut: &str) -> String {
    brut.chars()
        .filter(|caractere| caractere.is_ascii_alphanumeric())
        .map(|caractere| caractere.to_ascii_uppercase())
        .collect()
}

pub fn meme_poste(gauche: &str, droite: &str) -> bool {
    normaliser_poste(gauche) == normaliser_poste(droite)
}

/*
 * Base64url sans remplissage. Une licence se recopie a la main, se dicte et
 * voyage par courriel : l alphabet doit traverser tout cela sans caractere
 * special. Trente lignes ici valent mieux qu une dependance de plus.
 */
pub mod base64url {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    pub fn encoder(octets: &[u8]) -> String {
        let mut sortie = String::new();

        for morceau in octets.chunks(3) {
            let a = morceau[0] as u32;
            let b = morceau.get(1).copied().unwrap_or(0) as u32;
            let c = morceau.get(2).copied().unwrap_or(0) as u32;
            let groupe = (a << 16) | (b << 8) | c;

            sortie.push(ALPHABET[(groupe >> 18) as usize & 63] as char);
            sortie.push(ALPHABET[(groupe >> 12) as usize & 63] as char);

            if morceau.len() > 1 {
                sortie.push(ALPHABET[(groupe >> 6) as usize & 63] as char);
            }

            if morceau.len() > 2 {
                sortie.push(ALPHABET[groupe as usize & 63] as char);
            }
        }

        sortie
    }

    pub fn decodeur(texte: &str) -> Option<Vec<u8>> {
        let mut valeurs: Vec<u32> = Vec::with_capacity(texte.len());

        for caractere in texte.chars() {
            let octet = caractere as u8;
            let position = ALPHABET.iter().position(|connu| *connu == octet)?;

            if !caractere.is_ascii() {
                return None;
            }

            valeurs.push(position as u32);
        }

        let mut sortie: Vec<u8> = Vec::with_capacity(valeurs.len() * 3 / 4);

        for morceau in valeurs.chunks(4) {
            if morceau.len() == 1 {
                return None;
            }

            let groupe = (morceau[0] << 18)
                | (morceau[1] << 12)
                | (morceau.get(2).copied().unwrap_or(0) << 6)
                | morceau.get(3).copied().unwrap_or(0);

            sortie.push((groupe >> 16) as u8);

            if morceau.len() > 2 {
                sortie.push((groupe >> 8) as u8);
            }

            if morceau.len() > 3 {
                sortie.push(groupe as u8);
            }
        }

        Some(sortie)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Paire de test, sans aucun rapport avec la cle de production.
    const PRIVEE_TEST: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    const PUBLIQUE_TEST: &str =
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";

    fn charge_test() -> Charge {
        Charge {
            numero: "0001".to_string(),
            titulaire: "Boutique Chez Ali".to_string(),
            emise_le: "2026-10-24".to_string(),
            expire_le: Some("2027-10-24".to_string()),
            poste: None,
        }
    }

    #[test]
    fn signe_puis_verifie_sa_propre_licence() {
        let texte = signer(&charge_test(), PRIVEE_TEST).unwrap();
        let relue = verifier_avec(&texte, PUBLIQUE_TEST).unwrap();

        assert!(texte.starts_with("LCOS1."));
        assert_eq!(relue.titulaire, "Boutique Chez Ali");
        assert_eq!(relue.expire_le.as_deref(), Some("2027-10-24"));
        assert!(!relue.perpetuelle());
    }

    #[test]
    fn accepte_une_licence_collee_avec_des_espaces_et_des_retours() {
        let texte = signer(&charge_test(), PRIVEE_TEST).unwrap();
        let coupee = format!(
            "  {}\n{}  ",
            &texte[..40],
            &texte[40..]
        );

        assert!(verifier_avec(&coupee, PUBLIQUE_TEST).is_ok());
    }

    #[test]
    fn refuse_une_charge_modifiee() {
        let texte = signer(&charge_test(), PRIVEE_TEST).unwrap();
        let tranches: Vec<&str> = texte.split('.').collect();

        let mut charge = charge_test();

        charge.titulaire = "Un autre commerce".to_string();

        let charge_json = serde_json::to_vec(&charge).unwrap();
        let falsifiee = format!(
            "{}.{}.{}",
            tranches[0],
            base64url::encoder(&charge_json),
            tranches[2]
        );

        assert!(verifier_avec(&falsifiee, PUBLIQUE_TEST).is_err());
    }

    #[test]
    fn refuse_une_licence_signee_par_une_autre_cle() {
        let autre_privee = "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb";
        let texte = signer(&charge_test(), autre_privee).unwrap();

        assert!(verifier_avec(&texte, PUBLIQUE_TEST).is_err());
    }

    #[test]
    fn refuse_les_formes_douteuses() {
        assert!(verifier_avec("", PUBLIQUE_TEST).is_err());
        assert!(verifier_avec("   ", PUBLIQUE_TEST).is_err());
        assert!(verifier_avec("LCOS1.abc", PUBLIQUE_TEST).is_err());
        assert!(verifier_avec("LCOS0.abc.def", PUBLIQUE_TEST).is_err());
        assert!(verifier_avec("LCOS1.!!!.!!!", PUBLIQUE_TEST).is_err());
    }

    #[test]
    fn refuse_une_charge_sans_titulaire() {
        let mut charge = charge_test();

        charge.titulaire = "   ".to_string();

        let texte = signer(&charge, PRIVEE_TEST).unwrap();

        assert!(verifier_avec(&texte, PUBLIQUE_TEST).is_err());
    }

    #[test]
    fn porte_le_poste_quand_il_est_impose() {
        let mut charge = charge_test();

        charge.poste = Some("1A2B-3C4D-5E6F-7081".to_string());

        let texte = signer(&charge, PRIVEE_TEST).unwrap();
        let relue = verifier_avec(&texte, PUBLIQUE_TEST).unwrap();

        assert!(meme_poste(
            relue.poste.as_deref().unwrap(),
            "1a2b3c4d5e6f7081"
        ));
        assert!(!meme_poste(relue.poste.as_deref().unwrap(), "0000"));
    }

    #[test]
    fn met_en_forme_l_empreinte_du_poste() {
        assert_eq!(formater_poste("1a2b3c4d5e6f7081"), "1A2B-3C4D-5E6F-7081");
        assert_eq!(normaliser_poste(" 1a2b-3c4d "), "1A2B3C4D");
    }

    #[test]
    fn base64url_fait_l_aller_retour() {
        for taille in 0..32usize {
            let octets: Vec<u8> = (0..taille).map(|rang| (rang * 7 + 3) as u8).collect();
            let texte = base64url::encoder(&octets);

            assert_eq!(base64url::decodeur(&texte).unwrap(), octets);
            assert!(!texte.contains('='));
            assert!(!texte.contains('+'));
            assert!(!texte.contains('/'));
        }
    }

    #[test]
    fn base64url_refuse_un_caractere_etranger() {
        assert!(base64url::decodeur("abc$").is_none());
        assert!(base64url::decodeur("a").is_none());
    }
}
