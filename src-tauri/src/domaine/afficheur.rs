/*
 * Afficheur client : le petit ecran tourne vers le client, a l arriere de la
 * caisse. Il n affiche qu une somme, et c est tout ce qu on lui demande.
 *
 * Les afficheurs de ce marche sortent tous de la meme poignee d usines et
 * commutent entre plusieurs jeux de commandes, choisis par interrupteur sous
 * le boitier. Six sont couverts, et c est tout ce qui se rencontre :
 *   - texte   : le module accepte des caracteres bruts. Un saut de page remet
 *               l ecran a blanc avant chaque somme ;
 *   - cd5220  : jeu de commandes des afficheurs 2x20 les plus repandus
 *               (`ESC Q A` ecrit la ligne haute et l efface d abord) ;
 *   - escpos  : afficheur chaine derriere l imprimante thermique. `ESC = n`
 *               donne la main a l afficheur, sinon les octets partiraient sur
 *               le rouleau de papier ;
 *   - aedex   : `!#1` ecrit la ligne haute, `!#2` la basse ;
 *   - utc     : UTC Enhanced, `ESC u A` suivi de la ligne et d un retour ;
 *   - adm     : ADM 787/788, `SLE1` efface la ligne haute avant d y ecrire.
 *
 * Aucun de ces jeux ne se devine : le gerant les essaie l un apres l autre et
 * regarde l ecran. D ou l ordre de la liste, du plus courant au plus rare.
 *
 * Aucune dependance : ce sont des octets, et le port serie est un fichier.
 */
use serde::Serialize;

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
pub const PROTOCOLE_AEDEX: &str = "aedex";
pub const PROTOCOLE_UTC: &str = "utc";
pub const PROTOCOLE_ADM: &str = "adm";

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

/// `SLE1` d ADM 787/788 : efface la ligne haute et y ramene le curseur.
const EFFACE_LIGNE_HAUTE: u8 = 0x0E;

/*
 * `ESC = n` designe l appareil qui recevra la suite des octets. Deux
 * conventions coexistent pour `n`, et rien sur le boitier ne dit laquelle la
 * carte applique :
 *   - celle d Epson, d ou vient ESC/POS : `n` est un masque de bits, le bit 0
 *     pour l imprimante et le bit 1 pour l afficheur ;
 *   - celle des manuels d afficheurs VFD : `n` est un chiffre **ASCII**, '1'
 *     pour l imprimante, '2' pour l afficheur, '3' pour les deux.
 *
 * LCOS n envoyait que la premiere. Sur une carte qui attend la seconde, la
 * main ne passait jamais a l afficheur et la somme partait au rouleau de
 * papier : l ecran du client restait fige sur zero, sans qu aucun envoi
 * echoue.
 *
 * Les deux formes sont donc envoyees a la suite, l ASCII en dernier. `ESC =`
 * consomme toujours exactement un octet de parametre : une valeur que la carte
 * ne connait pas est avalee avec la commande, sans decaler le flux ni laisser
 * un octet s imprimer. La derniere forme comprise l emporte, et chaque forme
 * designe le meme appareil dans sa propre convention.
 */

/// Main donnee a l afficheur, dans les deux conventions.
const VERS_AFFICHEUR: [u8; 6] = [ECHAPPEMENT, b'=', 0x02, ECHAPPEMENT, b'=', b'2'];

/// Main rendue au rouleau de papier, dans les deux conventions.
const VERS_IMPRIMANTE: [u8; 6] = [ECHAPPEMENT, b'=', 0x01, ECHAPPEMENT, b'=', b'1'];

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
        PROTOCOLE_AEDEX => Ok(PROTOCOLE_AEDEX),
        PROTOCOLE_UTC => Ok(PROTOCOLE_UTC),
        PROTOCOLE_ADM => Ok(PROTOCOLE_ADM),
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
        // `!#1` vise la ligne haute. Le manuel annonce vingt caracteres, mais
        // le retour chariot termine la chaine : la largeur reglee fait foi.
        PROTOCOLE_AEDEX => {
            octets.extend_from_slice(b"!#1");
            octets.extend_from_slice(ligne.as_bytes());
            octets.push(RETOUR_CHARIOT);
        }
        PROTOCOLE_UTC => {
            octets.extend_from_slice(&[ECHAPPEMENT, b'u', b'A']);
            octets.extend_from_slice(ligne.as_bytes());
            octets.push(RETOUR_CHARIOT);
        }
        PROTOCOLE_ADM => {
            octets.push(EFFACE_LIGNE_HAUTE);
            octets.extend_from_slice(ligne.as_bytes());
        }
        PROTOCOLE_ESCPOS => {
            octets.extend_from_slice(&VERS_AFFICHEUR);
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
    let mut octets = VERS_AFFICHEUR.to_vec();

    octets.push(SAUT_DE_PAGE);
    octets.extend_from_slice(caler(texte, colonnes).as_bytes());
    octets.extend_from_slice(&VERS_IMPRIMANTE);

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

/*
 * ---------------------------------- sonde ----------------------------------
 *
 * Ecrire sur un port serie reussit toujours : le port s ouvre, les octets
 * partent, et rien ne dit si un afficheur est au bout du cable. Un envoi
 * reussi ne prouvait donc rien. La sonde cherche les trois seules preuves
 * qu un port serie peut donner :
 *   - le port existe et personne ne le tient (sinon l ouverture echoue) ;
 *   - les lignes de controle DSR et CTS sont hautes : un appareil est sous
 *     tension au bout du cable et le signale. La sonde leve donc DTR et RTS
 *     avant de les lire, sans quoi elles resteraient basses meme sur un cable
 *     branche : beaucoup de cables d afficheur se contentent de ponter DTR
 *     sur DSR et RTS sur CTS dans le connecteur ;
 *   - des octets reviennent : l afficheur accuse reception de la commande
 *     systeme que lui envoie `interrogation`.
 *
 * Aucune de ces preuves n est parfaite, et la sonde ne pretend pas le
 * contraire : un afficheur relie par un cable a trois fils (TX, RX, masse)
 * laisse DSR et CTS au repos tout en fonctionnant, et un modele d une autre
 * famille ignore la commande systeme. D ou le verdict `muet`, qui dit
 * l absence de preuve et non la preuve de l absence.
 *
 * Le port est lu par `System.IO.Ports.SerialPort` de Windows, a travers
 * PowerShell : c est le seul moyen de poser un delai d attente en lecture
 * sans ajouter de dependance, la ou un port ouvert comme un fichier bloque
 * indefiniment.
 */

/// Demande d etat en temps reel d ESC/POS : `DLE EOT n`.
const DEMANDE_ETAT: u8 = 0x10;
const FIN_TRANSMISSION: u8 = 0x04;

/// Enveloppe des commandes systeme des afficheurs VFD : `EOT SOH <ordre> ETB`.
/// Ces commandes ne dependent **d aucun mode d emulation** : elles sont
/// comprises que l afficheur soit regle en CD5220, en AEDEX ou en natif.
const DEBUT_ENTETE: u8 = 0x01;
const FIN_BLOC: u8 = 0x17;

/// `EOT SOH T ETB` : « transmets le message que tu affiches ». L afficheur
/// repond, et c est tout ce que la sonde demande.
const DEMANDE_AFFICHAGE: [u8; 4] = [FIN_TRANSMISSION, DEBUT_ENTETE, b'T', FIN_BLOC];

/// Delai d attente du port, en millisecondes. Un appareil qui repond le fait
/// en quelques dizaines de millisecondes : au-dela, il n y a personne.
const ATTENTE_SONDE_MS: i64 = 400;

/// Repos laisse a l appareil entre la demande et le releve de sa reponse.
const REPOS_SONDE_MS: i64 = 300;

pub const SONDE_REPOND: &str = "repond";
pub const SONDE_ALIMENTE: &str = "alimente";
pub const SONDE_MUET: &str = "muet";
pub const SONDE_ABSENT: &str = "absent";
pub const SONDE_OCCUPE: &str = "occupe";
pub const SONDE_BLOQUE: &str = "bloque";
pub const SONDE_INVALIDE: &str = "invalide";
pub const SONDE_ILLISIBLE: &str = "illisible";

#[derive(Debug, Serialize)]
pub struct Sonde {
    pub verdict: String,
    /// Vrai quand un appareil s est manifeste. Faux ne veut pas dire absent :
    /// il veut dire que rien ne l a prouve.
    pub prouve: bool,
    /// Lignes de controle du port au moment de la sonde.
    pub dsr: bool,
    pub cts: bool,
    pub octets_recus: i64,
    /// Phrase montree au gerant, deja ecrite dans ses termes.
    pub detail: String,
}

/// Octets a envoyer pour arracher une reponse a l appareil.
///
/// Deux cas, selon qui est au bout du cable :
///   - voie ESC/POS : c est la carte de l imprimante qui repond, interrogee
///     par sa demande d etat en temps reel `DLE EOT 1` ;
///   - port propre : c est l afficheur lui-meme, interroge par la commande
///     systeme `EOT SOH T ETB`, a laquelle il renvoie le message qu il
///     affiche. Cette commande ne depend pas du mode d emulation, donc la
///     sonde n a pas besoin de connaitre le bon protocole pour prouver la
///     presence de l appareil.
///
/// Un afficheur d un autre modele ignorera la demande. Le pire qu elle puisse
/// faire est de laisser un caractere sur l ecran, que le premier affichage
/// suivant effacera.
pub fn interrogation(protocole: &str) -> Vec<u8> {
    match protocole {
        PROTOCOLE_ESCPOS => vec![DEMANDE_ETAT, FIN_TRANSMISSION, 0x01],
        _ => DEMANDE_AFFICHAGE.to_vec(),
    }
}

/// Octets en hexadecimal separes par des virgules : la forme que la sonde
/// sait relire. Vide quand il n y a rien a envoyer.
pub fn hexadecimal(octets: &[u8]) -> String {
    octets
        .iter()
        .map(|octet| format!("{octet:02X}"))
        .collect::<Vec<String>>()
        .join(",")
}

/// Consigne PowerShell de la sonde. Elle rend une seule ligne :
/// `SONDE<tab>dsr<tab>cts<tab>octets` quand le port s est ouvert,
/// `ECHEC<tab>type` sinon. Le **type** de l exception est releve, jamais son
/// message : Windows le traduit dans la langue du poste.
///
/// Pas un seul guillemet double dans cette consigne, et aucune apostrophe
/// inverse : elle traverse la ligne de commande de `powershell.exe`, qui
/// mange les uns et interprete les autres. Les colonnes sont donc assemblees
/// par `-join [char]9`, et les chaines tenues entre apostrophes simples.
pub fn consigne_sonde(port: &str, debit: i64, interrogation: &[u8]) -> String {
    let demande = hexadecimal(interrogation);

    format!(
        "$ErrorActionPreference='Stop'; \
         function Cause($incident){{ $mal=$incident.Exception; \
         while($mal.InnerException){{ $mal=$mal.InnerException }}; $mal.GetType().FullName }} \
         $voie=$null; \
         try{{ \
         $voie=New-Object System.IO.Ports.SerialPort '{port}',{debit},'None',8,'One'; \
         $voie.Handshake='None'; $voie.DtrEnable=$true; $voie.RtsEnable=$true; \
         $voie.ReadTimeout={attente}; $voie.WriteTimeout={attente}; \
         $voie.Open(); \
         $dsr=$false; $cts=$false; \
         try{{ $dsr=$voie.DsrHolding }}catch{{}}; \
         try{{ $cts=$voie.CtsHolding }}catch{{}}; \
         $voie.DiscardInBuffer(); \
         $demande='{demande}'; \
         if($demande){{ \
         $octets=[byte[]]($demande -split ',' | ForEach-Object{{ [Convert]::ToByte($_,16) }}); \
         $voie.Write($octets,0,$octets.Length); Start-Sleep -Milliseconds {repos} }}; \
         ('SONDE',$dsr,$cts,$voie.BytesToRead) -join [char]9 \
         }}catch{{ ('ECHEC',(Cause $_)) -join [char]9 }}finally{{ \
         if($voie -and $voie.IsOpen){{ $voie.Close() }}; \
         if($voie){{ $voie.Dispose() }} }}",
        port = port,
        debit = debit,
        demande = demande,
        attente = ATTENTE_SONDE_MS,
        repos = REPOS_SONDE_MS
    )
}

/// Verdict tire de la ligne rendue par la sonde.
pub fn lire_sonde(sortie: &str) -> Sonde {
    let ligne = sortie
        .lines()
        .map(str::trim)
        .find(|ligne| !ligne.is_empty())
        .unwrap_or_default();

    let champs: Vec<&str> = ligne.split('\t').map(str::trim).collect();

    match champs.first().copied() {
        Some("SONDE") if champs.len() >= 4 => {
            let dsr = champs[1].eq_ignore_ascii_case("true");
            let cts = champs[2].eq_ignore_ascii_case("true");
            let octets_recus = champs[3].parse::<i64>().unwrap_or(0);

            ouvert(dsr, cts, octets_recus)
        }
        Some("ECHEC") if champs.len() >= 2 => refus(champs[1]),
        _ => sonde(
            SONDE_ILLISIBLE,
            false,
            "la sonde n a rien renvoye : Windows n a pas pu lire le port.",
        ),
    }
}

fn ouvert(dsr: bool, cts: bool, octets_recus: i64) -> Sonde {
    if octets_recus > 0 {
        return Sonde {
            octets_recus,
            dsr,
            cts,
            ..sonde(
                SONDE_REPOND,
                true,
                "l appareil a repondu : il y a bien du materiel au bout du cable.",
            )
        };
    }

    if dsr || cts {
        return Sonde {
            octets_recus,
            dsr,
            cts,
            ..sonde(
                SONDE_ALIMENTE,
                true,
                "aucune reponse, mais le port voit l appareil sous tension : il est branche et alimente.",
            )
        };
    }

    Sonde {
        octets_recus,
        dsr,
        cts,
        // Le detail est repris tel quel derriere un `mais` par l essai
        // d affichage : il commence donc sans conjonction.
        ..sonde(
            SONDE_MUET,
            false,
            "rien ne se manifeste sur ce port : afficheur eteint, debranche, \
             ou relie par un cable a trois fils qui ne porte pas les lignes de controle. \
             Regarde l ecran du client.",
        )
    }
}

/// Le type de l exception .NET dit precisement pourquoi le port s est refuse.
fn refus(cause: &str) -> Sonde {
    match cause {
        "System.IO.IOException" => sonde(
            SONDE_ABSENT,
            false,
            "ce port n existe pas sur cette machine : releve le materiel et prends un COM de la liste.",
        ),
        "System.UnauthorizedAccessException" => sonde(
            SONDE_OCCUPE,
            false,
            "le port existe mais un autre logiciel le tient : ferme-le, ou choisis un autre COM.",
        ),
        "System.TimeoutException" => sonde(
            SONDE_BLOQUE,
            false,
            "le port n a pas pris les octets : controle de flux bloque, ou appareil qui ne libere pas la ligne.",
        ),
        // Deux causes rendent le meme type : un nom qui n est pas un port
        // serie, et un port que Windows n arrive plus a configurer.
        //
        // Le second cas est un piege observe sur une caisse reelle : creer une
        // imprimante du spouleur sur un `COM` laisse le port ouvrable comme
        // fichier mais refuse par `GetCommState`, donc par la consigne `mode`
        // de `regler_port`. **Retirer l imprimante ne repare pas** : le port
        // reste inutilisable jusqu au redemarrage du poste. D ou le conseil de
        // ne jamais installer d imprimante sur le port de l afficheur.
        "System.ArgumentException" => sonde(
            SONDE_INVALIDE,
            false,
            "Windows n arrive pas a configurer ce port : soit le nom n est pas celui d un port \
             serie, soit une imprimante du spouleur a ete installee dessus et l a laisse dans \
             cet etat. Une imprimante et l afficheur ne peuvent pas partager un port, et \
             retirer l imprimante ne suffit pas : il faut redemarrer le poste.",
        ),
        autre => Sonde {
            detail: format!("le port s est refuse, Windows repond {autre}."),
            ..sonde(SONDE_ILLISIBLE, false, "")
        },
    }
}

fn sonde(verdict: &str, prouve: bool, detail: &str) -> Sonde {
    Sonde {
        verdict: verdict.to_string(),
        prouve,
        dsr: false,
        cts: false,
        octets_recus: 0,
        detail: detail.to_string(),
    }
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

        // La bascule part dans les deux conventions, l ASCII en dernier.
        assert_eq!(
            &octets[0..6],
            &[ECHAPPEMENT, b'=', 0x02, ECHAPPEMENT, b'=', 0x32]
        );
        assert_eq!(octets[6], SAUT_DE_PAGE);
        assert_eq!(&octets[7..], b"   15,00");
    }

    #[test]
    fn la_bascule_designe_le_meme_appareil_dans_les_deux_conventions() {
        // En convention ASCII, '2' nomme l afficheur ; en convention binaire,
        // 0x32 porte le bit 1, qui est celui de l afficheur. Les deux formes
        // ne peuvent donc pas se contredire, quelle que soit la carte.
        assert_eq!(VERS_AFFICHEUR[2] & 0x02, 0x02, "forme binaire");
        assert_eq!(VERS_AFFICHEUR[5], b'2');
        assert_eq!(VERS_AFFICHEUR[5] & 0x02, 0x02, "forme ASCII relue en bits");

        assert_eq!(VERS_IMPRIMANTE[2] & 0x01, 0x01, "forme binaire");
        assert_eq!(VERS_IMPRIMANTE[5], b'1');
        assert_eq!(VERS_IMPRIMANTE[5] & 0x01, 0x01, "forme ASCII relue en bits");

        // Aucune des deux formes ne vise l autre appareil.
        assert_eq!(VERS_AFFICHEUR[5] & 0x01, 0x00);
        assert_eq!(VERS_IMPRIMANTE[5] & 0x02, 0x00);
    }

    #[test]
    fn chaque_bascule_consomme_un_parametre_complet() {
        // Un `ESC =` prive de son octet de parametre avalerait l octet
        // suivant, qui est du texte : chaque forme fait donc trois octets, et
        // la paire exactement six.
        for bascule in [VERS_AFFICHEUR, VERS_IMPRIMANTE] {
            assert_eq!(bascule.len(), 6);
            assert_eq!(&bascule[0..2], &[ECHAPPEMENT, b'=']);
            assert_eq!(&bascule[3..5], &[ECHAPPEMENT, b'=']);
        }
    }

    #[test]
    fn la_trame_aedex_ecrit_la_ligne_haute() {
        let octets = trame("15,00", PROTOCOLE_AEDEX, 8);

        assert_eq!(&octets[0..3], b"!#1");
        assert_eq!(&octets[3..11], b"   15,00");
        assert_eq!(octets[11], RETOUR_CHARIOT);
    }

    #[test]
    fn la_trame_utc_ecrit_la_ligne_haute() {
        let octets = trame("15,00", PROTOCOLE_UTC, 8);

        assert_eq!(&octets[0..3], &[ECHAPPEMENT, b'u', b'A']);
        assert_eq!(&octets[3..11], b"   15,00");
        assert_eq!(octets[11], RETOUR_CHARIOT);
    }

    #[test]
    fn la_trame_adm_efface_avant_d_ecrire() {
        let octets = trame("15,00", PROTOCOLE_ADM, 8);

        assert_eq!(octets[0], EFFACE_LIGNE_HAUTE);
        assert_eq!(&octets[1..], b"   15,00");
    }

    #[test]
    fn chaque_protocole_efface_l_ecran_avant_d_ecrire() {
        // Sans effacement, la somme du client precedent reste derriere la
        // nouvelle quand celle-ci est plus courte : 9,00 sur 123,45 donnerait
        // 9,0045. Chaque protocole doit porter sa propre remise a blanc.
        for protocole in [
            PROTOCOLE_TEXTE,
            PROTOCOLE_CD5220,
            PROTOCOLE_ESCPOS,
            PROTOCOLE_AEDEX,
            PROTOCOLE_UTC,
            PROTOCOLE_ADM,
        ] {
            let octets = trame("9,00", protocole, 20);
            let ligne = caler("9,00", 20);
            let debut = octets
                .windows(ligne.len())
                .position(|fenetre| fenetre == ligne.as_bytes())
                .unwrap_or_else(|| panic!("somme absente de la trame {protocole}"));

            assert!(
                debut > 0,
                "{protocole} : la somme part sans commande qui efface"
            );
        }
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
        assert_eq!(&octets[0..6], &VERS_AFFICHEUR);
        assert_eq!(octets[6], SAUT_DE_PAGE);
        assert_eq!(&octets[7..15], b"   15,00");
        assert_eq!(&octets[15..], &VERS_IMPRIMANTE);
    }

    #[test]
    fn la_trame_imprimante_ne_laisse_jamais_la_main_a_l_afficheur() {
        // Oubliee, la main rendue ferait partir le ticket suivant sur
        // l afficheur, c est-a-dire nulle part.
        let octets = trame_imprimante("15,00", 20);

        assert!(octets.ends_with(&VERS_IMPRIMANTE));
    }

    #[test]
    fn n_accepte_que_les_voies_connues() {
        assert_eq!(valider_voie(" serie ").unwrap(), VOIE_SERIE);
        assert_eq!(valider_voie("imprimante").unwrap(), VOIE_IMPRIMANTE);
        assert!(valider_voie("usb").is_err());
    }

    #[test]
    fn n_accepte_que_les_protocoles_connus() {
        for protocole in [
            PROTOCOLE_TEXTE,
            PROTOCOLE_CD5220,
            PROTOCOLE_ESCPOS,
            PROTOCOLE_AEDEX,
            PROTOCOLE_UTC,
            PROTOCOLE_ADM,
        ] {
            assert_eq!(valider_protocole(protocole).unwrap(), protocole);
        }

        assert_eq!(valider_protocole(" escpos ").unwrap(), PROTOCOLE_ESCPOS);
        assert!(valider_protocole("vfd-maison").is_err());
    }

    #[test]
    fn escpos_interroge_la_carte_de_l_imprimante() {
        // `DLE EOT 1` : demande d etat en temps reel. C est l imprimante qui
        // porte l afficheur qui repond, pas l afficheur.
        assert_eq!(interrogation(PROTOCOLE_ESCPOS), vec![0x10, 0x04, 0x01]);
    }

    #[test]
    fn un_afficheur_a_son_port_est_interroge_par_la_commande_systeme() {
        // `EOT SOH T ETB` ne depend d aucun mode d emulation : la sonde prouve
        // donc la presence de l appareil sans connaitre son protocole.
        for protocole in [
            PROTOCOLE_TEXTE,
            PROTOCOLE_CD5220,
            PROTOCOLE_AEDEX,
            PROTOCOLE_UTC,
            PROTOCOLE_ADM,
        ] {
            assert_eq!(
                interrogation(protocole),
                vec![0x04, 0x01, 0x54, 0x17],
                "protocole : {protocole}"
            );
        }
    }

    #[test]
    fn la_demande_part_en_hexadecimal() {
        assert_eq!(hexadecimal(&[0x10, 0x04, 0x01]), "10,04,01");
        assert_eq!(hexadecimal(&[]), "");
    }

    #[test]
    fn la_consigne_de_sonde_porte_le_port_le_debit_et_la_demande() {
        let consigne = consigne_sonde("COM3", 9600, &interrogation(PROTOCOLE_ESCPOS));

        assert!(consigne.contains("SerialPort 'COM3',9600,'None',8,'One'"));
        assert!(consigne.contains("$demande='10,04,01'"));
        assert!(consigne.contains("$voie.ReadTimeout=400"));
        // Le message de l exception est traduit par Windows : seul son type
        // peut etre relu, et la consigne ne doit relever que lui.
        assert!(consigne.contains("$mal.GetType().FullName"));
    }

    #[test]
    fn la_consigne_de_sonde_ne_porte_aucun_guillemet_double() {
        // `powershell.exe` mange les guillemets doubles de sa ligne de
        // commande : la consigne assemble ses colonnes sans en utiliser un.
        let consigne = consigne_sonde("COM1", 9600, &interrogation(PROTOCOLE_ESCPOS));

        assert!(!consigne.contains('"'), "consigne : {consigne}");
        assert!(consigne.contains("-join [char]9"));
    }

    #[test]
    fn la_sonde_leve_dtr_et_rts_avant_de_lire_les_lignes() {
        // Beaucoup de cables d afficheur pontent DTR sur DSR et RTS sur CTS
        // dans le connecteur : lues sans avoir leve les sorties, les deux
        // entrees restent basses meme sur un cable branche, et la sonde
        // conclurait a tort au verdict `muet`.
        let consigne = consigne_sonde("COM2", 9600, &interrogation(PROTOCOLE_TEXTE));
        let leve = consigne.find("$voie.DtrEnable=$true").expect("DTR leve");
        let lu = consigne.find("$voie.DsrHolding").expect("DSR lu");

        assert!(leve < lu, "DTR doit etre leve avant la lecture de DSR");
        assert!(consigne.contains("$voie.RtsEnable=$true"));
    }

    #[test]
    fn la_sonde_envoie_la_commande_systeme_sur_un_port_propre() {
        let consigne = consigne_sonde("COM1", 9600, &interrogation(PROTOCOLE_TEXTE));

        assert!(consigne.contains("$demande='04,01,54,17'"));
    }

    #[test]
    fn des_octets_revenus_prouvent_la_presence() {
        let verdict = lire_sonde("SONDE\tFalse\tFalse\t4");

        assert_eq!(verdict.verdict, SONDE_REPOND);
        assert_eq!(verdict.octets_recus, 4);
        assert!(verdict.prouve);
    }

    #[test]
    fn une_ligne_de_controle_haute_prouve_l_alimentation() {
        let verdict = lire_sonde("SONDE\tTrue\tFalse\t0");

        assert_eq!(verdict.verdict, SONDE_ALIMENTE);
        assert!(verdict.dsr);
        assert!(!verdict.cts);
        assert!(verdict.prouve);
    }

    #[test]
    fn un_port_ouvert_mais_silencieux_ne_prouve_rien() {
        let verdict = lire_sonde("SONDE\tFalse\tFalse\t0");

        assert_eq!(verdict.verdict, SONDE_MUET);
        // Absence de preuve, pas preuve de l absence : un cable a trois fils
        // laisse DSR et CTS au repos alors que l afficheur fonctionne.
        assert!(!verdict.prouve);
    }

    #[test]
    fn traduit_le_refus_du_port_par_le_type_de_l_exception() {
        assert_eq!(lire_sonde("ECHEC\tSystem.IO.IOException").verdict, SONDE_ABSENT);
        assert_eq!(
            lire_sonde("ECHEC\tSystem.UnauthorizedAccessException").verdict,
            SONDE_OCCUPE
        );
        assert_eq!(
            lire_sonde("ECHEC\tSystem.TimeoutException").verdict,
            SONDE_BLOQUE
        );
        assert_eq!(
            lire_sonde("ECHEC\tSystem.ArgumentException").verdict,
            SONDE_INVALIDE
        );
    }

    #[test]
    fn une_sortie_inattendue_ne_passe_pas_pour_une_presence() {
        for sortie in ["", "\n\n", "SONDE\tTrue", "bruit", "ECHEC"] {
            let verdict = lire_sonde(sortie);

            assert_eq!(verdict.verdict, SONDE_ILLISIBLE, "sortie : {sortie:?}");
            assert!(!verdict.prouve);
        }
    }

    #[test]
    fn retient_la_premiere_ligne_utile_de_la_sonde() {
        let verdict = lire_sonde("\r\n  \r\nSONDE\tFalse\tTrue\t0\r\n");

        assert_eq!(verdict.verdict, SONDE_ALIMENTE);
        assert!(verdict.cts);
    }
}
