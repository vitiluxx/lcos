/*
 * Peripheriques de la caisse.
 *
 * Trois choses dans ce fichier :
 *   - les reglages de la douchette code-barres (cadence, longueur) ;
 *   - l inventaire du materiel que Windows connait, interne comme branche :
 *     c est la liste ou le gerant reconnait son afficheur client, sa douchette
 *     ou son tiroir ;
 *   - l afficheur client, le petit ecran tourne vers le client : son reglage
 *     et l envoi de la somme.
 *
 * L inventaire passe par le fournisseur CIM de Windows, comme l enumeration
 * des imprimantes : le materiel est deja decrit par le systeme, LCOS n a pas a
 * parler aux bus lui-meme.
 */
use std::io::Write;
use std::process::Command;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::depot::{parametres, Depot};
use crate::domaine::afficheur;
use crate::domaine::code_barres::{self, AnalyseCodeBarres};
use crate::domaine::monnaie;
use crate::erreur::{ErreurApplication, Resultat};
use crate::securite::jetons::ROLE_GERANT;
use crate::securite::RegistreJetons;

const CLE_ACTIF: &str = "scanner_actif";
const CLE_DELAI_MAX: &str = "scanner_delai_max_entre_touches";
const CLE_LONGUEUR_MIN: &str = "scanner_longueur_minimale";
const CLE_INACTIVITE: &str = "scanner_delai_inactivite";
const CLE_DETECTE: &str = "scanner_detecte";
const CLE_DERNIERE_DETECTION: &str = "scanner_derniere_detection";
const CLE_SIGNATURE: &str = "scanner_signature";

const CLE_AFFICHEUR_ACTIF: &str = "afficheur_actif";
const CLE_AFFICHEUR_PORT: &str = "afficheur_port";
const CLE_AFFICHEUR_DEBIT: &str = "afficheur_debit";
const CLE_AFFICHEUR_PROTOCOLE: &str = "afficheur_protocole";
const CLE_AFFICHEUR_COLONNES: &str = "afficheur_colonnes";
const CLE_AFFICHEUR_DERNIER_ENVOI: &str = "afficheur_dernier_envoi";

const CREATION_SANS_FENETRE: u32 = 0x0800_0000;

/// L inventaire d un poste Windows depasse facilement le millier de lignes
/// avec les pilotes systeme : au-dela, l ecran ne sert plus a rien.
const PERIPHERIQUES_LISTES: usize = 500;

const BORNES_DELAI_MAX: (i64, i64) = (5, 500);
const BORNES_LONGUEUR_MIN: (i64, i64) = (1, 64);
const BORNES_INACTIVITE: (i64, i64) = (20, 2000);
const LONGUEUR_SIGNATURE_MAXIMALE: usize = 120;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReglagesScanner {
    pub actif: bool,
    pub delai_max_entre_touches: i64,
    pub longueur_minimale: i64,
    pub delai_inactivite: i64,
    pub detecte: bool,
    pub derniere_detection: String,
    pub signature: String,
}

#[tauri::command]
pub fn recuperer_reglages_scanner(depot: State<Depot>) -> Resultat<ReglagesScanner> {
    depot.executer(charger)
}

#[tauri::command]
pub fn enregistrer_reglages_scanner(
    depot: State<Depot>,
    reglages: ReglagesScanner,
) -> Resultat<ReglagesScanner> {
    let delai_max = borner(reglages.delai_max_entre_touches, BORNES_DELAI_MAX, "delai entre touches")?;
    let longueur_min = borner(reglages.longueur_minimale, BORNES_LONGUEUR_MIN, "longueur minimale")?;
    let inactivite = borner(reglages.delai_inactivite, BORNES_INACTIVITE, "delai d inactivite")?;

    depot.executer(|connexion| {
        parametres::ecrire(connexion, CLE_ACTIF, booleen_vers_texte(reglages.actif))?;
        parametres::ecrire(connexion, CLE_DELAI_MAX, &delai_max.to_string())?;
        parametres::ecrire(connexion, CLE_LONGUEUR_MIN, &longueur_min.to_string())?;
        parametres::ecrire(connexion, CLE_INACTIVITE, &inactivite.to_string())?;

        charger(connexion)
    })
}

/// Enregistre la douchette comme reconnue : signature materielle et horodatage.
#[tauri::command]
pub fn enregistrer_detection_scanner(
    depot: State<Depot>,
    signature: String,
) -> Resultat<ReglagesScanner> {
    let signature_nettoyee = signature.trim();

    if signature_nettoyee.is_empty() {
        return Err(ErreurApplication::Validation("signature vide".to_string()));
    }

    if signature_nettoyee.len() > LONGUEUR_SIGNATURE_MAXIMALE {
        return Err(ErreurApplication::Validation(format!(
            "signature trop longue: maximum {LONGUEUR_SIGNATURE_MAXIMALE} caracteres"
        )));
    }

    depot.executer(|connexion| {
        parametres::ecrire(connexion, CLE_DETECTE, "oui")?;
        parametres::ecrire(connexion, CLE_SIGNATURE, signature_nettoyee)?;
        parametres::horodater(connexion, CLE_DERNIERE_DETECTION)?;

        charger(connexion)
    })
}

#[tauri::command]
pub fn analyser_code_barres(code: String) -> Resultat<AnalyseCodeBarres> {
    code_barres::analyser(&code)
}

/*
 * ------------------------------- inventaire -------------------------------
 */

#[derive(Debug, Serialize)]
pub struct Peripherique {
    pub nom: String,
    /// Famille donnee par Windows : `Ports`, `Monitor`, `HIDClass`, `USB`...
    pub famille: String,
    /// Comment l appareil tient a la machine : lu au debut de son identifiant
    /// systeme. C est ce qui separe le materiel branche du materiel integre.
    pub liaison: String,
    pub etat: String,
    pub fabricant: String,
    /// `COM3` quand l appareil se presente sur un port serie, vide sinon.
    pub port: String,
    pub identifiant: String,
}

#[derive(Debug, Serialize)]
pub struct Inventaire {
    pub releve_le: String,
    pub nombre: i64,
    pub tronque: bool,
    /// Ports serie libres de tout nom : c est la liste a proposer pour
    /// l afficheur client.
    pub ports_serie: Vec<String>,
    pub peripheriques: Vec<Peripherique>,
}

/// Tout ce que Windows connait comme materiel sur ce poste, interne comme
/// branche. Reserve au gerant : un identifiant de materiel decrit la machine.
#[tauri::command]
pub fn inventorier_peripheriques(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
) -> Resultat<Inventaire> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let mut peripheriques = enumerer_materiel()?;

    peripheriques.sort_by(|gauche, droite| {
        gauche
            .famille
            .to_lowercase()
            .cmp(&droite.famille.to_lowercase())
            .then(gauche.nom.to_lowercase().cmp(&droite.nom.to_lowercase()))
    });

    let nombre = peripheriques.len();
    let tronque = nombre > PERIPHERIQUES_LISTES;

    peripheriques.truncate(PERIPHERIQUES_LISTES);

    let mut ports_serie: Vec<String> = peripheriques
        .iter()
        .filter(|appareil| !appareil.port.is_empty())
        .map(|appareil| appareil.port.clone())
        .collect();

    ports_serie.sort();
    ports_serie.dedup();

    let releve_le = depot.executer(crate::depot::horloge::maintenant)?;

    Ok(Inventaire {
        releve_le: crate::domaine::horodatage::lisible(&releve_le),
        nombre: nombre as i64,
        tronque,
        ports_serie,
        peripheriques,
    })
}

/// `Win32_PnPEntity` liste le materiel que le systeme a monte, quel que soit
/// le bus : USB, PCI, ACPI, HID, Bluetooth. C est la meme vue que le
/// gestionnaire de peripheriques de Windows.
fn enumerer_materiel() -> Resultat<Vec<Peripherique>> {
    let consigne = "[Console]::OutputEncoding = [Text.Encoding]::UTF8; \
         Get-CimInstance -ClassName Win32_PnPEntity | ForEach-Object { \
         \"$($_.Name)`t$($_.PNPClass)`t$($_.Status)`t$($_.Manufacturer)`t$($_.PNPDeviceID)\" }";

    let sortie = lancer(&["-NoProfile", "-NonInteractive", "-Command", consigne])?;

    if !sortie.status.success() {
        let detail = String::from_utf8_lossy(&sortie.stderr);

        return Err(ErreurApplication::Conflit(format!(
            "materiel illisible : {}",
            detail.lines().next().unwrap_or("echec de l inventaire")
        )));
    }

    Ok(String::from_utf8_lossy(&sortie.stdout)
        .lines()
        .filter_map(analyser_materiel)
        .collect())
}

fn analyser_materiel(ligne: &str) -> Option<Peripherique> {
    let champs: Vec<&str> = ligne.trim_end_matches('\r').split('\t').collect();

    if champs.len() < 5 || champs[0].trim().is_empty() {
        return None;
    }

    let nom = champs[0].trim().to_string();
    let identifiant = champs[4].trim().to_string();

    Some(Peripherique {
        port: port_dans(&nom),
        famille: if champs[1].trim().is_empty() {
            "autre".to_string()
        } else {
            champs[1].trim().to_string()
        },
        liaison: liaison_de(&identifiant),
        etat: etat_de(champs[2].trim()),
        fabricant: champs[3].trim().to_string(),
        nom,
        identifiant,
    })
}

/// Windows nomme un port serie `Peripherique serie USB (COM3)` : le nom porte
/// le port, et c est la seule facon de le relier a un appareil.
fn port_dans(nom: &str) -> String {
    let Some(debut) = nom.rfind("(COM") else {
        return String::new();
    };

    let reste = &nom[debut + 1..];
    let Some(fin) = reste.find(')') else {
        return String::new();
    };

    afficheur::valider_port(&reste[..fin]).unwrap_or_default()
}

/// Premier element de l identifiant systeme : c est le bus qui porte
/// l appareil.
fn liaison_de(identifiant: &str) -> String {
    let bus = identifiant
        .split(['\\', '#'])
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();

    match bus.as_str() {
        "USB" | "USBSTOR" | "USBPRINT" => "USB (branche)".to_string(),
        "HID" => "HID (clavier, douchette, ecran tactile)".to_string(),
        "PCI" => "PCI (interne)".to_string(),
        "ACPI" => "ACPI (carte mere)".to_string(),
        "SCSI" | "SATA" | "IDE" | "NVME" => "disque (interne)".to_string(),
        "BTH" | "BTHENUM" | "BTHLE" => "Bluetooth".to_string(),
        "DISPLAY" | "MONITOR" => "ecran".to_string(),
        "SWD" | "ROOT" | "SW" => "logiciel (systeme)".to_string(),
        "" => "inconnue".to_string(),
        autre => autre.to_lowercase(),
    }
}

fn etat_de(statut: &str) -> String {
    match statut {
        "OK" => "en service".to_string(),
        "" => "inconnu".to_string(),
        "Error" | "Degraded" => "en defaut".to_string(),
        "Unknown" => "inconnu".to_string(),
        autre => autre.to_lowercase(),
    }
}

/*
 * ----------------------------- afficheur client ----------------------------
 */

#[derive(Debug, Serialize)]
pub struct ReglagesAfficheur {
    pub actif: bool,
    pub port: String,
    pub debit: i64,
    pub protocole: String,
    pub colonnes: i64,
    pub dernier_envoi: String,
}

#[tauri::command]
pub fn recuperer_reglages_afficheur(depot: State<Depot>) -> Resultat<ReglagesAfficheur> {
    depot.executer(charger_afficheur)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn enregistrer_reglages_afficheur(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    actif: bool,
    port: String,
    debit: i64,
    protocole: String,
    colonnes: i64,
) -> Resultat<ReglagesAfficheur> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let protocole_retenu = afficheur::valider_protocole(&protocole)?;
    let debit_retenu = afficheur::valider_debit(debit)?;
    let colonnes_retenues = afficheur::borner_colonnes(colonnes);

    // Un afficheur eteint n a pas besoin de port : c est le reglage d usine.
    let port_retenu = if actif {
        afficheur::valider_port(&port)?
    } else {
        port.trim().to_ascii_uppercase()
    };

    if actif {
        regler_port(&port_retenu, debit_retenu)?;
    }

    depot.executer(|connexion| {
        parametres::ecrire_booleen(connexion, CLE_AFFICHEUR_ACTIF, actif)?;
        parametres::ecrire(connexion, CLE_AFFICHEUR_PORT, &port_retenu)?;
        parametres::ecrire(connexion, CLE_AFFICHEUR_DEBIT, &debit_retenu.to_string())?;
        parametres::ecrire(connexion, CLE_AFFICHEUR_PROTOCOLE, protocole_retenu)?;
        parametres::ecrire(
            connexion,
            CLE_AFFICHEUR_COLONNES,
            &colonnes_retenues.to_string(),
        )?;

        charger_afficheur(connexion)
    })
}

/// Somme posee sur l ecran du client. Sans jeton : c est le caissier qui
/// encaisse, et l afficheur eteint rend simplement la main.
#[tauri::command]
pub fn afficher_somme_client(depot: State<Depot>, centimes: i64, decimales: i64) -> Resultat<bool> {
    let reglages = depot.executer(charger_afficheur)?;

    if !reglages.actif || reglages.port.is_empty() {
        return Ok(false);
    }

    let texte = monnaie::formater_avec(centimes, decimales);
    let octets = afficheur::trame(&texte, &reglages.protocole, reglages.colonnes);

    ecrire_sur_port(&reglages.port, &octets)?;

    depot.executer(|connexion| parametres::horodater(connexion, CLE_AFFICHEUR_DERNIER_ENVOI))?;

    Ok(true)
}

/// Ecran rendu a blanc entre deux clients.
#[tauri::command]
pub fn effacer_afficheur_client(depot: State<Depot>) -> Resultat<bool> {
    let reglages = depot.executer(charger_afficheur)?;

    if !reglages.actif || reglages.port.is_empty() {
        return Ok(false);
    }

    let octets = afficheur::trame_effacement(&reglages.protocole, reglages.colonnes);

    ecrire_sur_port(&reglages.port, &octets)?;

    Ok(true)
}

/// Essai depuis le parametrage : le gerant regarde l ecran arriere pendant que
/// LCOS y pose une somme. Les reglages passes ne sont pas encore enregistres,
/// pour essayer un port avant de le retenir.
#[tauri::command]
pub fn tester_afficheur(
    jetons: State<RegistreJetons>,
    jeton: String,
    port: String,
    debit: i64,
    protocole: String,
    colonnes: i64,
) -> Resultat<String> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let protocole_retenu = afficheur::valider_protocole(&protocole)?;
    let debit_retenu = afficheur::valider_debit(debit)?;
    let port_retenu = afficheur::valider_port(&port)?;
    let colonnes_retenues = afficheur::borner_colonnes(colonnes);

    regler_port(&port_retenu, debit_retenu)?;

    let octets = afficheur::trame("123,45", protocole_retenu, colonnes_retenues);

    ecrire_sur_port(&port_retenu, &octets)?;

    Ok(format!(
        "123,45 envoye sur {port_retenu} a {debit_retenu} bauds : regarde l ecran du client."
    ))
}

fn charger_afficheur(connexion: &rusqlite::Connection) -> Resultat<ReglagesAfficheur> {
    let valeurs = parametres::lire_prefixe(connexion, "afficheur_")?;

    let entier = |cle: &str, defaut: i64| -> i64 {
        valeurs
            .get(cle)
            .and_then(|valeur| valeur.parse::<i64>().ok())
            .unwrap_or(defaut)
    };

    let texte = |cle: &str| -> String { valeurs.get(cle).cloned().unwrap_or_default() };

    let protocole = texte(CLE_AFFICHEUR_PROTOCOLE);

    Ok(ReglagesAfficheur {
        actif: texte(CLE_AFFICHEUR_ACTIF) == "oui",
        port: texte(CLE_AFFICHEUR_PORT),
        debit: entier(CLE_AFFICHEUR_DEBIT, afficheur::DEBIT_DEFAUT),
        protocole: if protocole.is_empty() {
            afficheur::PROTOCOLE_TEXTE.to_string()
        } else {
            protocole
        },
        colonnes: afficheur::borner_colonnes(entier(
            CLE_AFFICHEUR_COLONNES,
            afficheur::COLONNES_DEFAUT,
        )),
        dernier_envoi: texte(CLE_AFFICHEUR_DERNIER_ENVOI),
    })
}

/// Le port est un fichier : ouvert, ecrit, referme. Un afficheur debranche
/// doit faire echouer l envoi, pas la vente, d ou le message explicite.
fn ecrire_sur_port(port: &str, octets: &[u8]) -> Resultat<()> {
    let chemin = afficheur::chemin_port(port);

    let mut fichier = std::fs::OpenOptions::new()
        .write(true)
        .open(&chemin)
        .map_err(|incident| {
            ErreurApplication::Conflit(format!(
                "afficheur client : {port} ne repond pas ({incident})"
            ))
        })?;

    fichier.write_all(octets)?;
    fichier.flush()?;

    Ok(())
}

/// Vitesse du port, posee par la consigne `mode` de Windows. Appelee a
/// l enregistrement et a l essai, jamais a chaque somme : un processus par
/// frappe du caissier serait du gaspillage.
fn regler_port(port: &str, debit: i64) -> Resultat<()> {
    let consigne = afficheur::consigne_mode(port, debit);
    let mut commande = Command::new("cmd");

    commande.args(["/C", &consigne]);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        commande.creation_flags(CREATION_SANS_FENETRE);
    }

    let sortie = commande.output()?;

    if !sortie.status.success() {
        return Err(ErreurApplication::Conflit(format!(
            "port {port} : vitesse refusee par Windows. Verifie que le port existe et qu aucun autre logiciel ne le tient."
        )));
    }

    Ok(())
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

fn charger(connexion: &rusqlite::Connection) -> Resultat<ReglagesScanner> {
    let valeurs = parametres::lire_prefixe(connexion, "scanner_")?;

    let entier = |cle: &str, defaut: i64| -> i64 {
        valeurs
            .get(cle)
            .and_then(|valeur| valeur.parse::<i64>().ok())
            .unwrap_or(defaut)
    };

    let texte = |cle: &str| -> String { valeurs.get(cle).cloned().unwrap_or_default() };

    Ok(ReglagesScanner {
        actif: texte(CLE_ACTIF) == "oui",
        delai_max_entre_touches: entier(CLE_DELAI_MAX, 50),
        longueur_minimale: entier(CLE_LONGUEUR_MIN, 4),
        delai_inactivite: entier(CLE_INACTIVITE, 120),
        detecte: texte(CLE_DETECTE) == "oui",
        derniere_detection: texte(CLE_DERNIERE_DETECTION),
        signature: texte(CLE_SIGNATURE),
    })
}

fn borner(valeur: i64, bornes: (i64, i64), libelle: &str) -> Resultat<i64> {
    let (minimum, maximum) = bornes;

    if valeur < minimum || valeur > maximum {
        return Err(ErreurApplication::Validation(format!(
            "{libelle}: {valeur} hors bornes [{minimum}, {maximum}]"
        )));
    }

    Ok(valeur)
}

fn booleen_vers_texte(valeur: bool) -> &'static str {
    if valeur {
        "oui"
    } else {
        "non"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_le_port_dans_le_nom_windows() {
        assert_eq!(port_dans("Peripherique serie USB (COM3)"), "COM3");
        assert_eq!(port_dans("Afficheur client (COM12)"), "COM12");
        assert_eq!(port_dans("Souris HID"), "");
        assert_eq!(port_dans("Carte (COMBO)"), "");
    }

    #[test]
    fn nomme_la_liaison_depuis_l_identifiant_systeme() {
        assert_eq!(liaison_de("USB\\VID_0416&PID_5011\\6&1"), "USB (branche)");
        assert_eq!(liaison_de("PCI\\VEN_8086"), "PCI (interne)");
        assert_eq!(liaison_de("HID\\VID_1234"), "HID (clavier, douchette, ecran tactile)");
        assert_eq!(liaison_de(""), "inconnue");
    }

    #[test]
    fn traduit_le_statut_de_windows() {
        assert_eq!(etat_de("OK"), "en service");
        assert_eq!(etat_de("Error"), "en defaut");
        assert_eq!(etat_de(""), "inconnu");
    }

    #[test]
    fn retient_une_ligne_cim_complete_et_ecarte_une_ligne_tronquee() {
        let ligne = "Afficheur client (COM3)\tPorts\tOK\tGeneric\tUSB\\VID_1A86&PID_7523";
        let appareil = analyser_materiel(ligne).expect("ligne complete");

        assert_eq!(appareil.nom, "Afficheur client (COM3)");
        assert_eq!(appareil.famille, "Ports");
        assert_eq!(appareil.port, "COM3");
        assert_eq!(appareil.liaison, "USB (branche)");
        assert_eq!(appareil.etat, "en service");

        assert!(analyser_materiel("Nom seul\tPorts").is_none());
        assert!(analyser_materiel("\t\t\t\t\t").is_none());
    }

    #[test]
    fn la_consigne_mode_porte_le_port_et_le_debit() {
        let consigne = afficheur::consigne_mode("COM3", 9600);

        assert!(consigne.starts_with("mode COM3: BAUD=9600"));
    }
}
