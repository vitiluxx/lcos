/*
 * Imprimante du ticket. Trois voies, de la plus simple a la plus directe :
 *   - navigateur : la webview ouvre la fenetre d impression de Windows ;
 *   - systeme    : le texte part au spouleur, sans boite de dialogue ;
 *   - escpos     : les octets ESC/POS partent au spouleur en mode brut (RAW),
 *                  seule voie qui coupe le papier, ouvre le tiroir-caisse et
 *                  met une ligne en gras sur une thermique 58 mm.
 *
 * Le mode escpos passe par l API du spouleur de Windows (winspool) : aucun
 * pilote tiers, aucune dependance ajoutee, le pilote installe par le
 * constructeur suffit des lors qu il accepte les donnees brutes.
 */
use std::io::Write;
use std::process::Command;

use serde::Serialize;
use tauri::State;

use crate::depot::{parametres, Depot};
use crate::domaine::escpos;
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::RegistreJetons;

const CLE_NOM: &str = "imprimante_nom";
const CLE_MODE: &str = "imprimante_mode";
const CLE_DETECTEE: &str = "imprimante_detectee";
const CLE_DERNIERE_DETECTION: &str = "imprimante_derniere_detection";
const CLE_COUPE: &str = "imprimante_coupe_papier";
const CLE_TIROIR: &str = "imprimante_tiroir_caisse";
const CLE_GRAS: &str = "imprimante_gras";

const MODE_NAVIGATEUR: &str = "navigateur";
const MODE_SYSTEME: &str = "systeme";
const MODE_ESCPOS: &str = "escpos";

const CREATION_SANS_FENETRE: u32 = 0x0800_0000;
const LONGUEUR_NOM_MAXIMALE: usize = 120;
const LONGUEUR_TEXTE_MAXIMALE: usize = 20_000;
const FICHIER_TAMPON: &str = "lcos_ticket.txt";

/// Prefixes de port qui menent a du materiel. Tout le reste est un port
/// logiciel : boite de dialogue, fichier, telecopie, ou port applicatif d une
/// application du Store.
const PORTS_MATERIELS: [&str; 5] = ["COM", "LPT", "USB", "IP_", "WSD-"];

#[derive(Debug, Serialize)]
pub struct Imprimante {
    pub nom: String,
    pub par_defaut: bool,
    pub hors_ligne: bool,
    pub etat: String,
    /// Port du spouleur : `COM2:`, `LPT1:`, `USB001`, `PORTPROMPT:`...
    pub port: String,
    pub pilote: String,
    /// Vrai quand le port ne mene a aucun materiel. Ces imprimantes se
    /// declarent `prete` comme les autres : sans ce drapeau, le gerant choisit
    /// `Microsoft Print to PDF` en croyant retenir la thermique de la caisse.
    pub virtuelle: bool,
}

#[derive(Debug, Serialize)]
pub struct ReglagesImpression {
    pub mode: String,
    pub imprimante: String,
    pub detectee: bool,
    pub derniere_detection: String,
    pub coupe_papier: bool,
    pub tiroir_caisse: bool,
    pub gras: bool,
    pub disponibles: Vec<Imprimante>,
}

#[tauri::command]
pub fn recuperer_reglages_impression(depot: State<Depot>) -> Resultat<ReglagesImpression> {
    retenir(&depot, enumerer().unwrap_or_default())
}

/// Relance l enumeration du spouleur : utile juste apres avoir branche ou
/// allume l imprimante.
#[tauri::command]
pub fn detecter_imprimantes(depot: State<Depot>) -> Resultat<ReglagesImpression> {
    retenir(&depot, enumerer()?)
}

/// Une seule interrogation du spouleur par appel : son enumeration coute un
/// processus Windows.
fn retenir(depot: &Depot, disponibles: Vec<Imprimante>) -> Resultat<ReglagesImpression> {
    depot.executer(move |connexion| {
        memoriser_detection(connexion, &disponibles)?;

        charger(connexion, disponibles)
    })
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn enregistrer_reglages_impression(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    mode: String,
    imprimante: String,
    coupe_papier: bool,
    tiroir_caisse: bool,
    gras: bool,
) -> Resultat<ReglagesImpression> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let mode_retenu = valider_mode(&mode)?;
    let nom = valider_nom(&imprimante)?;

    if mode_retenu != MODE_NAVIGATEUR && nom.is_empty() {
        return Err(ErreurApplication::Validation(
            "envoi direct : choisir une imprimante dans la liste".to_string(),
        ));
    }

    depot.executer(|connexion| {
        parametres::ecrire(connexion, CLE_MODE, mode_retenu)?;
        parametres::ecrire(connexion, CLE_NOM, &nom)?;
        parametres::ecrire_booleen(connexion, CLE_COUPE, coupe_papier)?;
        parametres::ecrire_booleen(connexion, CLE_TIROIR, tiroir_caisse)?;
        parametres::ecrire_booleen(connexion, CLE_GRAS, gras)?;

        charger(connexion, enumerer().unwrap_or_default())
    })
}

/// Envoi au spouleur Windows. Le front n appelle cette commande que lorsque le
/// mode systeme ou escpos est retenu ; sinon il imprime par la webview.
#[tauri::command]
pub fn imprimer_ticket(depot: State<Depot>, texte: String) -> Resultat<String> {
    if texte.trim().is_empty() {
        return Err(ErreurApplication::Validation(
            "rien a imprimer : texte vide".to_string(),
        ));
    }

    if texte.len() > LONGUEUR_TEXTE_MAXIMALE {
        return Err(ErreurApplication::Validation(format!(
            "ticket trop long : maximum {LONGUEUR_TEXTE_MAXIMALE} caracteres"
        )));
    }

    let (nom, mode, options) = depot.executer(|connexion| {
        Ok((
            parametres::lire_ou(connexion, CLE_NOM, "")?,
            parametres::lire_ou(connexion, CLE_MODE, MODE_NAVIGATEUR)?,
            lire_options(connexion)?,
        ))
    })?;

    if nom.trim().is_empty() {
        return Err(ErreurApplication::Conflit(
            "aucune imprimante retenue : choisir l imprimante dans Parametrage".to_string(),
        ));
    }

    if mode == MODE_ESCPOS {
        brut::envoyer(&nom, &escpos::encoder(&texte, &options))?;
    } else {
        envoyer_texte(&nom, &texte)?;
    }

    Ok(nom)
}

/// Resultat d un essai d impression. Le spouleur accepte les octets sans rien
/// savoir de ce qui est au bout du cable : `materiel` dit seulement si le port
/// retenu peut porter un appareil, jamais qu un ticket est sorti.
#[derive(Debug, Serialize)]
pub struct EssaiImpression {
    pub imprimante: String,
    pub port: String,
    pub materiel: bool,
    pub message: String,
}

#[tauri::command]
pub fn tester_impression(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<EssaiImpression> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let nom = imprimer_ticket(depot, page_de_test())?;

    // L enumeration est relancee apres l envoi : c est elle qui porte le port
    // de l imprimante retenue, et le reglage ne le memorise pas.
    let (port, materiel) = enumerer()
        .unwrap_or_default()
        .into_iter()
        .find(|imprimante| imprimante.nom == nom)
        .map(|imprimante| (imprimante.port, !imprimante.virtuelle))
        .unwrap_or_default();

    let message = if materiel {
        format!(
            "Page de test remise au spouleur pour {nom} sur {port}. \
             Windows accepte les octets meme sans appareil au bout du cable : \
             regarde le rouleau, c est lui qui tranche."
        )
    } else {
        format!(
            "Page de test remise au spouleur pour {nom}, mais son port ({}) ne mene a aucun \
             materiel : aucun ticket ne sortira. Retiens l imprimante de la caisse, \
             celle dont le port commence par COM, LPT ou USB.",
            if port.is_empty() { "inconnu" } else { port.as_str() }
        )
    };

    Ok(EssaiImpression {
        imprimante: nom,
        port,
        materiel,
        message,
    })
}

/// Octets bruts vers une imprimante du spouleur, partage avec l afficheur
/// client : sur une caisse tout-en-un, l afficheur est chaine derriere
/// l imprimante et n a pas de port a lui.
pub(crate) fn envoyer_octets(imprimante: &str, octets: &[u8]) -> Resultat<()> {
    brut::envoyer(imprimante, octets)
}

/// Imprimante retenue pour le ticket. Vide si le gerant n en a choisi aucune.
pub(crate) fn imprimante_retenue(connexion: &rusqlite::Connection) -> Resultat<String> {
    parametres::lire_ou(connexion, CLE_NOM, "")
}

fn lire_options(connexion: &rusqlite::Connection) -> Resultat<escpos::Options> {
    Ok(escpos::Options {
        gras: parametres::lire_booleen(connexion, CLE_GRAS, true)?,
        coupe: parametres::lire_booleen(connexion, CLE_COUPE, true)?,
        tiroir: parametres::lire_booleen(connexion, CLE_TIROIR, false)?,
    })
}

fn charger(
    connexion: &rusqlite::Connection,
    disponibles: Vec<Imprimante>,
) -> Resultat<ReglagesImpression> {
    let options = lire_options(connexion)?;

    Ok(ReglagesImpression {
        mode: parametres::lire_ou(connexion, CLE_MODE, MODE_NAVIGATEUR)?,
        imprimante: parametres::lire_ou(connexion, CLE_NOM, "")?,
        detectee: parametres::lire_booleen(connexion, CLE_DETECTEE, false)?,
        derniere_detection: parametres::lire_ou(connexion, CLE_DERNIERE_DETECTION, "")?,
        coupe_papier: options.coupe,
        tiroir_caisse: options.tiroir,
        gras: options.gras,
        disponibles,
    })
}

/// Premiere imprimante vue : on la retient comme imprimante du ticket pour
/// que le gerant n ait rien a choisir dans le cas courant d un seul materiel.
fn memoriser_detection(
    connexion: &rusqlite::Connection,
    disponibles: &[Imprimante],
) -> Resultat<()> {
    if disponibles.is_empty() {
        parametres::ecrire_booleen(connexion, CLE_DETECTEE, false)?;

        return Ok(());
    }

    parametres::ecrire_booleen(connexion, CLE_DETECTEE, true)?;
    parametres::horodater(connexion, CLE_DERNIERE_DETECTION)?;

    let retenue = parametres::lire_ou(connexion, CLE_NOM, "")?;
    let connue = disponibles
        .iter()
        .any(|imprimante| imprimante.nom == retenue);

    if !retenue.trim().is_empty() && connue {
        return Ok(());
    }

    let defaut = disponibles
        .iter()
        .find(|imprimante| imprimante.par_defaut)
        .unwrap_or(&disponibles[0]);

    parametres::ecrire(connexion, CLE_NOM, &defaut.nom)
}

/// Le port et le pilote sont releves en plus de l etat : l etat du spouleur ne
/// distingue pas une thermique branchee d une imprimante virtuelle, les deux se
/// declarent `prete`. Le port, lui, le dit.
///
/// Colonnes assemblees par `-join [char]9`, sans guillemet double : la consigne
/// traverse la ligne de commande de `powershell.exe`, qui les mange.
const CONSIGNE_SPOULEUR: &str = "[Console]::OutputEncoding = [Text.Encoding]::UTF8; \
     Get-CimInstance -ClassName Win32_Printer | ForEach-Object { \
     ($_.Name,$_.Default,$_.WorkOffline,$_.PrinterStatus,$_.PortName,$_.DriverName) \
     -join [char]9 }";

fn enumerer() -> Resultat<Vec<Imprimante>> {
    let sortie = lancer(&["-NoProfile", "-NonInteractive", "-Command", CONSIGNE_SPOULEUR])?;

    if !sortie.status.success() {
        let detail = String::from_utf8_lossy(&sortie.stderr);

        return Err(ErreurApplication::Conflit(format!(
            "spouleur Windows illisible : {}",
            detail.lines().next().unwrap_or("echec de l enumeration")
        )));
    }

    Ok(String::from_utf8_lossy(&sortie.stdout)
        .lines()
        .filter_map(analyser_ligne)
        .collect())
}

fn analyser_ligne(ligne: &str) -> Option<Imprimante> {
    let champs: Vec<&str> = ligne.trim_end_matches('\r').split('\t').collect();

    if champs.len() < 4 || champs[0].trim().is_empty() {
        return None;
    }

    let port = champs.get(4).unwrap_or(&"").trim().to_string();

    Some(Imprimante {
        nom: champs[0].trim().to_string(),
        par_defaut: champs[1].eq_ignore_ascii_case("true"),
        hors_ligne: champs[2].eq_ignore_ascii_case("true"),
        etat: libelle_etat(champs[3].trim()),
        pilote: champs.get(5).unwrap_or(&"").trim().to_string(),
        virtuelle: !mene_a_du_materiel(&port),
        port,
    })
}

/// Un port qui mene a du materiel : serie, parallele, USB ou reseau.
///
/// Rien dans le spouleur ne dit si un appareil est reellement au bout du
/// cable — `PrinterStatus` vaut `prete` sur un port vide comme sur un port
/// equipe, verifie sur sept ports dont quatre sans aucun materiel. Le nom du
/// port est donc la seule information exploitable : il ne prouve pas la
/// presence, mais il prouve l absence.
fn mene_a_du_materiel(port: &str) -> bool {
    let majuscules = port.trim().to_ascii_uppercase();

    PORTS_MATERIELS
        .iter()
        .any(|prefixe| majuscules.starts_with(prefixe))
}

/// Codes PrinterStatus de Win32_Printer, traduits pour l ecran du gerant.
fn libelle_etat(code: &str) -> String {
    match code {
        "1" => "autre",
        "2" => "inconnu",
        "3" => "prete",
        "4" => "impression",
        "5" => "prechauffage",
        "6" => "arretee",
        "7" => "hors service",
        _ => "indeterminee",
    }
    .to_string()
}

/// Mode systeme : le texte passe par un fichier tampon, donc aucun guillemet
/// ni retour chariot ne traverse la ligne de commande PowerShell.
fn envoyer_texte(imprimante: &str, texte: &str) -> Resultat<()> {
    let chemin = std::env::temp_dir().join(FICHIER_TAMPON);

    {
        let mut fichier = std::fs::File::create(&chemin)?;

        fichier.write_all(texte.replace('\r', "").as_bytes())?;
    }

    let consigne = format!(
        "$texte = [IO.File]::ReadAllText('{}', [Text.Encoding]::UTF8); \
         $texte -split \"`n\" | Out-Printer -Name '{}'",
        echapper(&chemin.to_string_lossy()),
        echapper(imprimante)
    );

    let sortie = lancer(&["-NoProfile", "-NonInteractive", "-Command", &consigne])?;

    let _ = std::fs::remove_file(&chemin);

    if !sortie.status.success() {
        let detail = String::from_utf8_lossy(&sortie.stderr);

        return Err(ErreurApplication::Conflit(format!(
            "impression refusee par Windows : {}",
            detail.lines().next().unwrap_or("echec de l envoi")
        )));
    }

    Ok(())
}

/// Chaine PowerShell entre apostrophes : seule l apostrophe est a doubler.
fn echapper(valeur: &str) -> String {
    valeur.replace('\'', "''")
}

fn lancer(arguments: &[&str]) -> Resultat<std::process::Output> {
    let mut commande = Command::new("powershell");
    commande.args(arguments);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        commande.creation_flags(CREATION_SANS_FENETRE);
    }

    Ok(commande.output()?)
}

fn valider_mode(mode: &str) -> Resultat<&'static str> {
    match mode.trim() {
        MODE_ESCPOS => Ok(MODE_ESCPOS),
        MODE_SYSTEME => Ok(MODE_SYSTEME),
        MODE_NAVIGATEUR => Ok(MODE_NAVIGATEUR),
        autre => Err(ErreurApplication::Validation(format!(
            "mode d impression inconnu : {autre}"
        ))),
    }
}

fn valider_nom(nom: &str) -> Resultat<String> {
    let nettoye = nom.trim();

    if nettoye.len() > LONGUEUR_NOM_MAXIMALE {
        return Err(ErreurApplication::Validation(format!(
            "nom d imprimante trop long : maximum {LONGUEUR_NOM_MAXIMALE} caracteres"
        )));
    }

    if nettoye.chars().any(|caractere| caractere.is_control()) {
        return Err(ErreurApplication::Validation(
            "nom d imprimante : caracteres de controle refuses".to_string(),
        ));
    }

    Ok(nettoye.to_string())
}

fn page_de_test() -> String {
    [
        "LCOS",
        "--------------------------------",
        "Page de test imprimante",
        "",
        "01234567890123456789012345678901",
        "Si cette ligne tient sur une",
        "seule ligne, la largeur est bonne.",
        "",
        "TOTAL test                  0,00",
        "--------------------------------",
    ]
    .join("\n")
}

/*
 * Envoi brut au spouleur de Windows (datatype RAW). C est le seul chemin qui
 * laisse passer les octets de controle ESC/POS : la fenetre d impression et
 * Out-Printer passent par le rendu graphique du pilote, qui les avalerait.
 */
#[cfg(windows)]
mod brut {
    use std::ffi::c_void;
    use std::ptr;

    use crate::erreur::{ErreurApplication, Resultat};

    type Poignee = *mut c_void;

    const NIVEAU_INFO_DOCUMENT: u32 = 1;

    #[repr(C)]
    struct InfoDocument {
        nom: *const u16,
        fichier_sortie: *const u16,
        type_donnees: *const u16,
    }

    #[link(name = "winspool")]
    extern "system" {
        fn OpenPrinterW(nom: *const u16, poignee: *mut Poignee, defauts: *mut c_void) -> i32;
        fn ClosePrinter(poignee: Poignee) -> i32;
        fn StartDocPrinterW(poignee: Poignee, niveau: u32, info: *const InfoDocument) -> u32;
        fn EndDocPrinter(poignee: Poignee) -> i32;
        fn StartPagePrinter(poignee: Poignee) -> i32;
        fn EndPagePrinter(poignee: Poignee) -> i32;
        fn WritePrinter(
            poignee: Poignee,
            octets: *const u8,
            taille: u32,
            ecrits: *mut u32,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetLastError() -> u32;
    }

    /// Ferme la poignee du spouleur quel que soit le chemin de sortie, y
    /// compris en cas d erreur au milieu du document.
    struct Fermeture(Poignee);

    impl Drop for Fermeture {
        fn drop(&mut self) {
            unsafe {
                ClosePrinter(self.0);
            }
        }
    }

    fn utf16(valeur: &str) -> Vec<u16> {
        valeur.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn echec(etape: &str) -> ErreurApplication {
        let code = unsafe { GetLastError() };

        ErreurApplication::Conflit(format!(
            "envoi ESC/POS refuse par Windows ({etape}, code {code})"
        ))
    }

    pub fn envoyer(imprimante: &str, octets: &[u8]) -> Resultat<()> {
        let nom = utf16(imprimante);
        let mut poignee: Poignee = ptr::null_mut();

        if unsafe { OpenPrinterW(nom.as_ptr(), &mut poignee, ptr::null_mut()) } == 0 {
            return Err(echec("ouverture de l imprimante"));
        }

        let _fermeture = Fermeture(poignee);

        let titre = utf16("Ticket LCOS");
        let type_donnees = utf16("RAW");
        let info = InfoDocument {
            nom: titre.as_ptr(),
            fichier_sortie: ptr::null(),
            type_donnees: type_donnees.as_ptr(),
        };

        if unsafe { StartDocPrinterW(poignee, NIVEAU_INFO_DOCUMENT, &info) } == 0 {
            return Err(echec("ouverture du document"));
        }

        if unsafe { StartPagePrinter(poignee) } == 0 {
            unsafe { EndDocPrinter(poignee) };

            return Err(echec("ouverture de la page"));
        }

        let mut ecrits: u32 = 0;
        let pose = unsafe {
            WritePrinter(
                poignee,
                octets.as_ptr(),
                octets.len() as u32,
                &mut ecrits,
            )
        };

        unsafe {
            EndPagePrinter(poignee);
            EndDocPrinter(poignee);
        }

        if pose == 0 {
            return Err(echec("ecriture des octets"));
        }

        if ecrits as usize != octets.len() {
            return Err(ErreurApplication::Conflit(format!(
                "envoi ESC/POS incomplet : {ecrits} octets sur {}",
                octets.len()
            )));
        }

        Ok(())
    }
}

/// Hors Windows, LCOS n a pas de spouleur a qui parler : le mode escpos n est
/// pas proposable. Le code reste compilable pour les tests.
#[cfg(not(windows))]
mod brut {
    use crate::erreur::{ErreurApplication, Resultat};

    pub fn envoyer(_imprimante: &str, _octets: &[u8]) -> Resultat<()> {
        Err(ErreurApplication::Conflit(
            "envoi ESC/POS disponible sous Windows uniquement".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyse_une_ligne_du_spouleur() {
        let imprimante =
            analyser_ligne("POS58\tTrue\tFalse\t3\tCOM2:\tPOS58 Driver").unwrap();

        assert_eq!(imprimante.nom, "POS58");
        assert!(imprimante.par_defaut);
        assert!(!imprimante.hors_ligne);
        assert_eq!(imprimante.etat, "prete");
        assert_eq!(imprimante.port, "COM2:");
        assert_eq!(imprimante.pilote, "POS58 Driver");
        assert!(!imprimante.virtuelle);
    }

    #[test]
    fn reconnait_une_imprimante_virtuelle_malgre_son_etat_pret() {
        // `PrinterStatus` vaut 3 = prete sur les deux lignes : seul le port
        // les distingue.
        let pdf = analyser_ligne(
            "Microsoft Print to PDF\tFalse\tFalse\t3\tPORTPROMPT:\tMicrosoft Print To PDF",
        )
        .unwrap();

        assert_eq!(pdf.etat, "prete");
        assert!(pdf.virtuelle);
    }

    #[test]
    fn lit_une_ligne_de_l_ancien_format_sans_port() {
        // Un releve sans les deux colonnes ajoutees reste exploitable, mais
        // sans port connu l imprimante est tenue pour virtuelle : mieux vaut
        // la signaler a tort que laisser croire a un ticket qui ne sort pas.
        let imprimante = analyser_ligne("POS58\tTrue\tFalse\t3").unwrap();

        assert_eq!(imprimante.port, "");
        assert!(imprimante.virtuelle);
    }

    #[test]
    fn distingue_les_ports_materiels_des_ports_logiciels() {
        for port in ["COM1:", "com2:", "LPT1:", "USB001", "IP_192.168.1.50", "WSD-abc"] {
            assert!(mene_a_du_materiel(port), "port materiel : {port}");
        }

        for port in [
            "PORTPROMPT:",
            "SHRFAX:",
            "FILE:",
            "nul:",
            "Microsoft.Office.OneNote_16001_x64__8wekyb3d8bbwe_microsoft.onenoteim",
            "",
            "   ",
        ] {
            assert!(!mene_a_du_materiel(port), "port logiciel : {port}");
        }
    }

    #[test]
    fn la_consigne_du_spouleur_ne_porte_aucun_guillemet_double() {
        // Meme lecon que la sonde de l afficheur : `powershell.exe` mange les
        // guillemets doubles de sa ligne de commande.
        assert!(!CONSIGNE_SPOULEUR.contains('"'));
        assert!(CONSIGNE_SPOULEUR.contains("-join [char]9"));
        assert!(CONSIGNE_SPOULEUR.contains("$_.PortName"));
    }

    #[test]
    fn ignore_une_ligne_incomplete() {
        assert!(analyser_ligne("").is_none());
        assert!(analyser_ligne("POS58\tTrue").is_none());
        assert!(analyser_ligne("\tTrue\tFalse\t3").is_none());
    }

    #[test]
    fn double_les_apostrophes_powershell() {
        assert_eq!(echapper("Imprimante d'appoint"), "Imprimante d''appoint");
    }

    #[test]
    fn refuse_un_mode_inconnu() {
        assert!(valider_mode("reseau").is_err());
        assert_eq!(valider_mode(" systeme ").unwrap(), MODE_SYSTEME);
        assert_eq!(valider_mode("escpos").unwrap(), MODE_ESCPOS);
    }

    #[test]
    fn refuse_un_nom_de_controle() {
        assert!(valider_nom("POS\n58").is_err());
        assert_eq!(valider_nom("  POS58  ").unwrap(), "POS58");
    }

    #[test]
    fn la_page_de_test_exerce_le_gras_et_la_largeur() {
        let page = page_de_test();

        assert!(page.contains("01234567890123456789012345678901"));
        assert!(page.lines().any(|ligne| ligne.starts_with("TOTAL")));
    }
}
