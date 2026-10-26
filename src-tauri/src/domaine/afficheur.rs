/*
 * Afficheur client : le petit ecran tourne vers le client, a l arriere de la
 * caisse. Il n affiche qu une somme, et c est tout ce qu on lui demande.
 *
 * Trois protocoles couvrent le materiel qui se branche sur une caisse de ce
 * type :
 *   - texte   : le module accepte des caracteres bruts. Un saut de page remet
 *               l ecran a blanc avant chaque somme ;
 *   - cd5220  : jeu de commandes des afficheurs 2x20 les plus repandus
 *               (`ESC Q A` ecrit la ligne haute et l efface d abord) ;
 *   - escpos  : afficheur chaine derriere l imprimante thermique. `ESC = 2`
 *               donne la main a l afficheur, sinon les octets partiraient sur
 *               le rouleau de papier.
 *
 * Aucune dependance : ce sont des octets, et le port serie est un fichier.
 */
use crate::erreur::{ErreurApplication, Resultat};

/*
 * Deux voies pour atteindre l afficheur :
 *   - serie      : l afficheur a son propre port, meme quand la prise est de
 *                  l USB. Windows le montre alors comme un `COM` ;
 *   - imprimante : l afficheur est chaine derriere l imprimante thermique et
 *                  n a aucun port a lui. Les octets partent au spouleur comme
 *                  un ticket, precedes de la commande qui donne la main a
 *                  l afficheur et suivis de celle qui la rend au rouleau.
 *
 * La seconde voie est celle des caisses tout-en-un : la carte de l imprimante
 * porte une prise pour l afficheur client, et rien n apparait dans les ports
 * de Windows.
 */
pub const VOIE_SERIE: &str = "serie";
pub const VOIE_IMPRIMANTE: &str = "imprimante";

pub const PROTOCOLE_TEXTE: &str = "texte";
pub const PROTOCOLE_CD5220: &str = "cd5220";
pub const PROTOCOLE_ESCPOS: &str = "escpos";

pub const COLONNES_DEFAUT: i64 = 20;
const COLONNES_MINIMALES: i64 = 8;
const COLONNES_MAXIMALES: i64 = 40;

pub const DEBIT_DEFAUT: i64 = 9600;
const DEBITS_CONNUS: [i64; 6] = [1200, 2400, 4800, 9600, 19200, 38400];

/// Numero de port le plus eleve que Windows attribue.
const PORT_MAXIMAL: u32 = 256;

const SAUT_DE_PAGE: u8 = 0x0C;
const RETOUR_CHARIOT: u8 = 0x0D;
const ECHAPPEMENT: u8 = 0x1B;

pub fn borner_colonnes(colonnes: i64) -> i64 {
    colonnes.clamp(COLONNES_MINIMALES, COLONNES_MAXIMALES)
}

pub fn valider_voie(voie: &str) -> Resultat<&'static str> {
    match voie.trim() {
        VOIE_SERIE => Ok(VOIE_SERIE),
        VOIE_IMPRIMANTE => Ok(VOIE_IMPRIMANTE),
        autre => Err(ErreurApplication::Validation(format!(
            "voie d afficheur inconnue : {autre}"
        ))),
    }
}

pub fn valider_protocole(protocole: &str) -> Resultat<&'static str> {
    match protocole.trim() {
        PROTOCOLE_TEXTE => Ok(PROTOCOLE_TEXTE),
        PROTOCOLE_CD5220 => Ok(PROTOCOLE_CD5220),
        PROTOCOLE_ESCPOS => Ok(PROTOCOLE_ESCPOS),
        autre => Err(ErreurApplication::Validation(format!(
            "protocole d afficheur inconnu : {autre}"
        ))),
    }
}

pub fn valider_debit(debit: i64) -> Resultat<i64> {
    if DEBITS_CONNUS.contains(&debit) {
        return Ok(debit);
    }

    Err(ErreurApplication::Validation(format!(
        "debit {debit} inconnu : {:?} attendus",
        DEBITS_CONNUS
    )))
}

/// Un nom de port serie, et rien d autre. L afficheur est ecrit comme un
/// fichier : sans ce filtre, le reglage designerait n importe quel chemin du
/// disque.
pub fn valider_port(port: &str) -> Resultat<String> {
    let nettoye = port.trim().trim_end_matches(':').to_ascii_uppercase();
    let reste = nettoye.strip_prefix("COM").unwrap_or("");

    let numero: u32 = reste
        .parse()
        .map_err(|_| ErreurApplication::Validation(
            "port de l afficheur : COM1 a COM256 attendu".to_string(),
        ))?;

    if numero == 0 || numero > PORT_MAXIMAL {
        return Err(ErreurApplication::Validation(
            "port de l afficheur : COM1 a COM256 attendu".to_string(),
        ));
    }

    Ok(format!("COM{numero}"))
}

/// Chemin d ouverture du port. Au-dela de COM9, Windows n accepte plus le nom
/// court : seule la forme `\\.\COMnn` ouvre le port.
pub fn chemin_port(port: &str) -> String {
    format!("\\\\.\\{port}")
}

/// Somme calee a droite sur la largeur de l ecran : les chiffres restent
/// alignes d une vente a l autre, le client lit toujours au meme endroit.
pub fn caler(texte: &str, colonnes: i64) -> String {
    let largeur = borner_colonnes(colonnes) as usize;
    let propre: String = texte
        .chars()
        .filter(|caractere| caractere.is_ascii_graphic() || *caractere == ' ')
        .collect();

    if propre.chars().count() >= largeur {
        return propre.chars().skip(propre.chars().count() - largeur).collect();
    }

    let manque = largeur - propre.chars().count();

    format!("{}{propre}", " ".repeat(manque))
}

/// Octets a ecrire sur le port pour poser une somme sur l ecran.
pub fn trame(texte: &str, protocole: &str, colonnes: i64) -> Vec<u8> {
    let ligne = caler(texte, colonnes);
    let mut octets = Vec::new();

    match protocole {
        PROTOCOLE_CD5220 => {
            octets.extend_from_slice(&[ECHAPPEMENT, b'Q', b'A']);
            octets.extend_from_slice(ligne.as_bytes());
            octets.push(RETOUR_CHARIOT);
        }
        PROTOCOLE_ESCPOS => {
            octets.extend_from_slice(&[ECHAPPEMENT, b'=', 0x02]);
            octets.push(SAUT_DE_PAGE);
            octets.extend_from_slice(ligne.as_bytes());
        }
        _ => {
            octets.push(SAUT_DE_PAGE);
            octets.extend_from_slice(ligne.as_bytes());
        }
    }

    octets
}

/// Octets a envoyer au spouleur quand l afficheur est chaine derriere
/// l imprimante. La main est **rendue au rouleau** a la fin : sans cela, le
/// ticket suivant s imprimerait sur l afficheur, c est-a-dire nulle part.
pub fn trame_imprimante(texte: &str, colonnes: i64) -> Vec<u8> {
    let mut octets = vec![ECHAPPEMENT, b'=', 0x02, SAUT_DE_PAGE];

    octets.extend_from_slice(caler(texte, colonnes).as_bytes());
    octets.extend_from_slice(&[ECHAPPEMENT, b'=', 0x01]);

    octets
}

/// Ecran rendu a blanc : fin de vente, le montant du client precedent ne doit
/// pas rester devant le suivant.
pub fn trame_effacement(protocole: &str, colonnes: i64) -> Vec<u8> {
    trame("", protocole, colonnes)
}

/// Consigne `mode` de Windows qui fixe la vitesse du port. Le pilote du
/// constructeur garde parfois un debit d usine : sans cette passe, l ecran
/// affiche des caracteres au hasard.
pub fn consigne_mode(port: &str, debit: i64) -> String {
    format!("mode {port}: BAUD={debit} PARITY=N DATA=8 STOP=1 to=on xon=off odsr=off octs=off dtr=on rts=on idsr=off")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn n_accepte_que_des_ports_serie() {
        assert_eq!(valider_port("com3").unwrap(), "COM3");
        assert_eq!(valider_port(" COM12: ").unwrap(), "COM12");
        assert!(valider_port("COM0").is_err());
        assert!(valider_port("COM999").is_err());
        assert!(valider_port("LPT1").is_err());
        assert!(valider_port("C:\\Windows\\system32\\cmd.exe").is_err());
        assert!(valider_port("").is_err());
    }

    #[test]
    fn ouvre_les_ports_hauts_par_leur_chemin_long() {
        assert_eq!(chemin_port("COM12"), "\\\\.\\COM12");
    }

    #[test]
    fn cale_la_somme_a_droite() {
        assert_eq!(caler("1 500,00", 12), "    1 500,00");
        assert_eq!(caler("", 8), "        ");
    }

    #[test]
    fn tronque_par_la_gauche_une_somme_trop_longue() {
        // Les centimes valent mieux que les millions coupes : un ecran trop
        // etroit garde la fin du nombre.
        assert_eq!(caler("1234567890123", 8), "67890123");
    }

    #[test]
    fn ecarte_les_caracteres_que_l_ecran_ne_sait_pas_rendre() {
        assert_eq!(caler("15,00\n\r", 8), "   15,00");
    }

    #[test]
    fn la_trame_texte_efface_avant_d_ecrire() {
        let octets = trame("15,00", PROTOCOLE_TEXTE, 8);

        assert_eq!(octets[0], SAUT_DE_PAGE);
        assert_eq!(&octets[1..], b"   15,00");
    }

    #[test]
    fn la_trame_cd5220_ecrit_la_ligne_haute() {
        let octets = trame("15,00", PROTOCOLE_CD5220, 8);

        assert_eq!(&octets[0..3], &[ECHAPPEMENT, b'Q', b'A']);
        assert_eq!(&octets[3..11], b"   15,00");
        assert_eq!(octets[11], RETOUR_CHARIOT);
    }

    #[test]
    fn la_trame_escpos_donne_la_main_a_l_afficheur() {
        let octets = trame("15,00", PROTOCOLE_ESCPOS, 8);

        assert_eq!(&octets[0..3], &[ECHAPPEMENT, b'=', 0x02]);
        assert_eq!(octets[3], SAUT_DE_PAGE);
        assert_eq!(&octets[4..], b"   15,00");
    }

    #[test]
    fn l_effacement_ne_laisse_que_des_espaces() {
        let octets = trame_effacement(PROTOCOLE_TEXTE, 10);

        assert_eq!(&octets[1..], b"          ");
    }

    #[test]
    fn borne_la_largeur_et_le_debit() {
        assert_eq!(borner_colonnes(0), COLONNES_MINIMALES);
        assert_eq!(borner_colonnes(500), COLONNES_MAXIMALES);
        assert_eq!(borner_colonnes(20), 20);
        assert!(valider_debit(9600).is_ok());
        assert!(valider_debit(7777).is_err());
    }

    #[test]
    fn la_trame_imprimante_rend_la_main_au_rouleau() {
        let octets = trame_imprimante("15,00", 8);

        // Main a l afficheur, ecran efface, somme, main rendue a l imprimante.
        assert_eq!(&octets[0..4], &[ECHAPPEMENT, b'=', 0x02, SAUT_DE_PAGE]);
        assert_eq!(&octets[4..12], b"   15,00");
        assert_eq!(&octets[12..], &[ECHAPPEMENT, b'=', 0x01]);
    }

    #[test]
    fn n_accepte_que_les_voies_connues() {
        assert_eq!(valider_voie(" serie ").unwrap(), VOIE_SERIE);
        assert_eq!(valider_voie("imprimante").unwrap(), VOIE_IMPRIMANTE);
        assert!(valider_voie("usb").is_err());
    }

    #[test]
    fn n_accepte_que_les_protocoles_connus() {
        assert_eq!(valider_protocole(" escpos ").unwrap(), PROTOCOLE_ESCPOS);
        assert!(valider_protocole("vfd-maison").is_err());
    }
}
