/*
 * Peripheriques de la caisse.
 *
 * Trois choses dans ce fichier :
 *   - les reglages de la douchette code-barres (cadence, longueur) ;
 *   - l inventaire du materiel que Windows connait, interne comme branche :
 *     c est la liste ou le gerant reconnait son afficheur client, sa douchette
 *     ou son tiroir. Il porte aussi les ecrans reellement branches et les
 *     douchettes candidates, pour que le gerant n ait rien a deviner ;
 *   - l afficheur client, le petit ecran tourne vers le client : son reglage,
 *     l envoi de la somme, et la sonde qui cherche a prouver sa presence.
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
use crate::domaine::afficheur::{self, Sonde};
use crate::domaine::code_barres::{self, AnalyseCodeBarres};
use crate::domaine::ecran::{self, Ecran};
use crate::domaine::monnaie;
use crate::domaine::usb;
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
const CLE_MATERIEL: &str = "scanner_materiel";

const CLE_AFFICHEUR_ACTIF: &str = "afficheur_actif";
const CLE_AFFICHEUR_VOIE: &str = "afficheur_voie";
const CLE_AFFICHEUR_PORT: &str = "afficheur_port";
const CLE_AFFICHEUR_DEBIT: &str = "afficheur_debit";
const CLE_AFFICHEUR_PROTOCOLE: &str = "afficheur_protocole";
const CLE_AFFICHEUR_COLONNES: &str = "afficheur_colonnes";
const CLE_AFFICHEUR_DERNIER_ENVOI: &str = "afficheur_dernier_envoi";

const CREATION_SANS_FENETRE: u32 = 0x0800_0000;

/// L inventaire d un poste Windows depasse facilement le millier de lignes
/// avec les pilotes systeme : au-dela, l ecran ne sert plus a rien.
const PERIPHERIQUES_LISTES: usize = 500;

/// Verdicts de sonde qui rendent l envoi inutile ou dangereux.
const VERDICTS_INJOIGNABLES: [&str; 4] = [
    afficheur::SONDE_ABSENT,
    afficheur::SONDE_OCCUPE,
    afficheur::SONDE_INVALIDE,
    afficheur::SONDE_BLOQUE,
];

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
    /// Signature de comportement : suffixe, longueur, cadence. Elle dit
    /// comment la douchette tape, pas quel appareil elle est.
    pub signature: String,
    /// Identite materielle `VID:PID` de la douchette retenue. C est elle qui
    /// survit a un changement de port ou a un redemarrage. Vide tant que le
    /// gerant n a pas designe l appareil.
    #[serde(default)]
    pub materiel: String,
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

/// Enregistre la douchette comme reconnue : signature de comportement,
/// horodatage, et identite materielle quand elle est connue sans ambiguite.
///
/// Sans jeton : la detection se fait au premier scan, avant toute ouverture de
/// session, et c est tout l interet.
#[tauri::command]
pub fn enregistrer_detection_scanner(
    depot: State<Depot>,
    signature: String,
    materiel: Option<String>,
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

    let identite = usb::valider_identite(&materiel.unwrap_or_default())?;

    depot.executer(|connexion| {
        parametres::ecrire(connexion, CLE_DETECTE, "oui")?;
        parametres::ecrire(connexion, CLE_SIGNATURE, signature_nettoyee)?;
        parametres::horodater(connexion, CLE_DERNIERE_DETECTION)?;

        // Une identite deja retenue n est pas ecrasee par un scan : seul le
        // gerant la change, par `designer_douchette`.
        if !identite.is_empty() && parametres::lire_ou(connexion, CLE_MATERIEL, "")?.is_empty() {
            parametres::ecrire(connexion, CLE_MATERIEL, &identite)?;
        }

        charger(connexion)
    })
}

/// Douchette designee par le gerant parmi les candidats de l inventaire.
/// Une identite vide efface la designation.
#[tauri::command]
pub fn designer_douchette(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    materiel: String,
) -> Resultat<ReglagesScanner> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let identite = usb::valider_identite(&materiel)?;

    depot.executer(|connexion| {
        parametres::ecrire(connexion, CLE_MATERIEL, &identite)?;

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

/// Une douchette possible. Un lecteur en mode clavier se declare toujours
/// comme un clavier : c est le seul filtre qui ait un sens ici, puisque LCOS
/// ne lit les codes-barres que par les frappes.
#[derive(Debug, Serialize)]
pub struct CandidatDouchette {
    /// `VID:PID`, l identite stable de l appareil.
    pub materiel: String,
    pub nom: String,
    /// Nombre d interfaces que l appareil presente a Windows. Un clavier de
    /// caisse en expose plusieurs (touches, pave, lecteur de carte) ; une
    /// douchette n en a le plus souvent qu une ou deux.
    pub interfaces: i64,
}

#[derive(Debug, Serialize)]
pub struct Inventaire {
    pub releve_le: String,
    pub nombre: i64,
    pub tronque: bool,
    /// Ports serie libres de tout nom : c est la liste a proposer pour
    /// l afficheur client.
    pub ports_serie: Vec<String>,
    /// Vrais moniteurs branches. L afficheur client n y figure jamais : voir
    /// `domaine::ecran`.
    pub ecrans: Vec<Ecran>,
    /// Appareils qui pourraient etre la douchette, pour que le gerant la
    /// designe au lieu de la deviner.
    pub douchettes: Vec<CandidatDouchette>,
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

    // Les candidats sont tires de la liste entiere, avant l ecourtement :
    // la douchette ne doit pas disparaitre parce que le poste porte mille
    // pilotes.
    let douchettes = candidats_douchette(&peripheriques);

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
        ecrans: relever_ecrans()?,
        douchettes,
        peripheriques,
    })
}

/// Les ecrans que Windows gere reellement, avec leur rang et leur resolution.
/// `Win32_PnPEntity` dit qu un moniteur existe ; lui seul ne dit pas lequel
/// est celui du caissier.
fn relever_ecrans() -> Resultat<Vec<Ecran>> {
    let sortie = lancer(&["-NoProfile", "-NonInteractive", "-Command", ecran::CONSIGNE])?;

    // Un releve d ecrans rate n est pas une raison de perdre l inventaire :
    // le materiel liste reste la seule information indispensable.
    if !sortie.status.success() {
        return Ok(Vec::new());
    }

    Ok(ecran::lire(&String::from_utf8_lossy(&sortie.stdout)))
}

/// Appareils qui presentent un clavier a Windows, regroupes par identite
/// materielle. La douchette de la caisse est forcement l un d eux ; le clavier
/// du caissier aussi, d ou le choix laisse au gerant.
fn candidats_douchette(peripheriques: &[Peripherique]) -> Vec<CandidatDouchette> {
    let mut candidats: Vec<CandidatDouchette> = Vec::new();

    for appareil in peripheriques {
        let Some(materiel) = usb::identite(&appareil.identifiant) else {
            continue;
        };

        let clavier = est_un_clavier(appareil);
        let connu = candidats
            .iter()
            .position(|candidat| candidat.materiel == materiel);

        let Some(rang) = connu else {
            candidats.push(CandidatDouchette {
                materiel,
                nom: appareil.nom.clone(),
                interfaces: 1,
            });

            continue;
        };

        candidats[rang].interfaces += 1;

        // Le nom de l interface clavier est le plus parlant des noms qu un
        // appareil porte : il remplace celui retenu avant lui.
        if clavier {
            candidats[rang].nom = appareil.nom.clone();
        }
    }

    // Seuls les appareils qui tapent des touches peuvent etre lus par LCOS :
    // une douchette reglee en mode serie ou HID-POS n arrive jamais par les
    // frappes, et la proposer serait mentir.
    let claviers: Vec<String> = peripheriques
        .iter()
        .filter(|appareil| est_un_clavier(appareil))
        .filter_map(|appareil| usb::identite(&appareil.identifiant))
        .collect();

    candidats.retain(|candidat| claviers.contains(&candidat.materiel));

    // Le plus probable en tete : une douchette expose moins d interfaces qu un
    // clavier de caisse, qui porte aussi son pave et parfois un lecteur de
    // carte.
    candidats.sort_by(|gauche, droite| {
        gauche
            .interfaces
            .cmp(&droite.interfaces)
            .then(gauche.nom.to_lowercase().cmp(&droite.nom.to_lowercase()))
    });

    candidats
}

fn est_un_clavier(appareil: &Peripherique) -> bool {
    appareil.famille.eq_ignore_ascii_case("Keyboard")
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
    /// `serie` : l afficheur a son propre port COM. `imprimante` : il est
    /// chaine derriere l imprimante thermique et n a pas de port a lui.
    pub voie: String,
    pub port: String,
    pub debit: i64,
    pub protocole: String,
    pub colonnes: i64,
    pub dernier_envoi: String,
    /// Imprimante retenue pour le ticket : c est elle qui porte l afficheur
    /// quand la voie est `imprimante`. Vide si aucune n est choisie.
    pub imprimante: String,
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
    voie: String,
    port: String,
    debit: i64,
    protocole: String,
    colonnes: i64,
) -> Resultat<ReglagesAfficheur> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let voie_retenue = afficheur::valider_voie(&voie)?;
    let protocole_retenu = afficheur::valider_protocole(&protocole)?;
    let debit_retenu = afficheur::valider_debit(debit)?;
    let colonnes_retenues = afficheur::borner_colonnes(colonnes);
    let par_le_port = actif && voie_retenue == afficheur::VOIE_SERIE;

    // Un afficheur eteint, ou branche derriere l imprimante, n a pas de port a
    // lui : le reglage garde la valeur saisie sans l exiger.
    let port_retenu = if par_le_port {
        afficheur::valider_port(&port)?
    } else {
        port.trim().to_ascii_uppercase()
    };

    if par_le_port {
        regler_port(&port_retenu, debit_retenu)?;
    }

    if actif && voie_retenue == afficheur::VOIE_IMPRIMANTE {
        depot.executer(exiger_imprimante)?;
    }

    depot.executer(|connexion| {
        parametres::ecrire_booleen(connexion, CLE_AFFICHEUR_ACTIF, actif)?;
        parametres::ecrire(connexion, CLE_AFFICHEUR_VOIE, voie_retenue)?;
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

    if !pret(&reglages) {
        return Ok(false);
    }

    let texte = monnaie::formater_avec(centimes, decimales);

    envoyer(&reglages, &texte)?;

    depot.executer(|connexion| parametres::horodater(connexion, CLE_AFFICHEUR_DERNIER_ENVOI))?;

    Ok(true)
}

/// Ecran rendu a blanc entre deux clients.
#[tauri::command]
pub fn effacer_afficheur_client(depot: State<Depot>) -> Resultat<bool> {
    let reglages = depot.executer(charger_afficheur)?;

    if !pret(&reglages) {
        return Ok(false);
    }

    envoyer(&reglages, "")?;

    Ok(true)
}

/// Un afficheur allume et joignable : la voie serie demande un port, la voie
/// imprimante demande une imprimante retenue.
fn pret(reglages: &ReglagesAfficheur) -> bool {
    if !reglages.actif {
        return false;
    }

    if reglages.voie == afficheur::VOIE_IMPRIMANTE {
        return !reglages.imprimante.is_empty();
    }

    !reglages.port.is_empty()
}

/// Seul endroit qui sait par ou passent les octets.
fn envoyer(reglages: &ReglagesAfficheur, texte: &str) -> Resultat<()> {
    if reglages.voie == afficheur::VOIE_IMPRIMANTE {
        let octets = afficheur::trame_imprimante(texte, reglages.colonnes);

        return super::impression::envoyer_octets(&reglages.imprimante, &octets);
    }

    let octets = afficheur::trame(texte, &reglages.protocole, reglages.colonnes);

    ecrire_sur_port(&reglages.port, &octets)
}

fn exiger_imprimante(connexion: &rusqlite::Connection) -> Resultat<String> {
    let nom = super::impression::imprimante_retenue(connexion)?;

    if nom.trim().is_empty() {
        return Err(ErreurApplication::Conflit(
            "afficheur derriere l imprimante : choisis d abord l imprimante du ticket, plus haut dans cet ecran".to_string(),
        ));
    }

    Ok(nom)
}

/*
 * -------------------------------- la sonde ---------------------------------
 */

/// Cherche a prouver qu un afficheur est au bout du cable, sans rien afficher.
/// Reserve au gerant : la sonde ouvre un port et manipule ses lignes.
#[tauri::command]
pub fn sonder_afficheur(
    jetons: State<RegistreJetons>,
    jeton: String,
    port: String,
    debit: i64,
    protocole: String,
) -> Resultat<Sonde> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let port_retenu = afficheur::valider_port(&port)?;
    let debit_retenu = afficheur::valider_debit(debit)?;
    let protocole_retenu = afficheur::valider_protocole(&protocole)?;

    sonder(&port_retenu, debit_retenu, protocole_retenu)
}

fn sonder(port: &str, debit: i64, protocole: &str) -> Resultat<Sonde> {
    let consigne = afficheur::consigne_sonde(port, debit, &afficheur::interrogation(protocole));
    let sortie = lancer(&["-NoProfile", "-NonInteractive", "-Command", &consigne])?;

    // La consigne rattrape ses propres incidents et sort toujours en reussite :
    // une sortie vide ne peut venir que d un PowerShell absent, et `lire_sonde`
    // la traduit en verdict `illisible`.
    Ok(afficheur::lire_sonde(&String::from_utf8_lossy(
        &sortie.stdout,
    )))
}

/// Resultat d un essai d affichage. Le message se lit tel quel ; `prouve` dit
/// s il faut le presenter comme une reussite ou comme un simple envoi, car un
/// envoi reussi sur un port serie ne prouve rien par lui-meme.
#[derive(Debug, Serialize)]
pub struct Essai {
    pub message: String,
    /// Verdict de la sonde qui a precede l envoi. Vide pour la voie
    /// imprimante : il n y a pas de port a sonder derriere un spouleur.
    pub verdict: String,
    pub prouve: bool,
}

/// Verdicts qui rendent l envoi inutile ou dangereux. `bloque` en fait partie :
/// la vitesse du port est posee avec `to=on`, donc une ligne tenue par
/// l appareil ferait attendre l ecriture sans fin.
fn port_injoignable(verdict: &str) -> bool {
    VERDICTS_INJOIGNABLES.contains(&verdict)
}

/// Essai depuis le parametrage : le gerant regarde l ecran arriere pendant que
/// LCOS y pose une somme. Les reglages passes ne sont pas encore enregistres,
/// pour essayer un port avant de le retenir.
///
/// Le port est sonde avant l envoi. Ecrire sur un port serie reussit toujours,
/// meme sans rien au bout du cable : sans cette passe, l essai annoncait une
/// reussite qui ne prouvait rien.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn tester_afficheur(
    depot: State<Depot>,
    jetons: State<RegistreJetons>,
    jeton: String,
    voie: String,
    port: String,
    debit: i64,
    protocole: String,
    colonnes: i64,
) -> Resultat<Essai> {
    jetons.exiger(&jeton, ROLE_GERANT)?;

    let voie_retenue = afficheur::valider_voie(&voie)?;
    let protocole_retenu = afficheur::valider_protocole(&protocole)?;
    let colonnes_retenues = afficheur::borner_colonnes(colonnes);

    if voie_retenue == afficheur::VOIE_IMPRIMANTE {
        let imprimante = depot.executer(exiger_imprimante)?;
        let octets = afficheur::trame_imprimante("123,45", colonnes_retenues);

        super::impression::envoyer_octets(&imprimante, &octets)?;

        // Le spouleur accepte les octets sans rien dire de ce qui est au bout :
        // rien n est prouve ici non plus.
        return Ok(Essai {
            message: format!("123,45 envoye a travers {imprimante} : regarde l ecran du client."),
            verdict: String::new(),
            prouve: false,
        });
    }

    let debit_retenu = afficheur::valider_debit(debit)?;
    let port_retenu = afficheur::valider_port(&port)?;

    let sonde = sonder(&port_retenu, debit_retenu, protocole_retenu)?;

    if port_injoignable(&sonde.verdict) {
        return Err(ErreurApplication::Conflit(format!(
            "{port_retenu} : {}",
            sonde.detail
        )));
    }

    regler_port(&port_retenu, debit_retenu)?;

    let octets = afficheur::trame("123,45", protocole_retenu, colonnes_retenues);

    ecrire_sur_port(&port_retenu, &octets)?;

    let message = if sonde.prouve {
        format!(
            "Afficheur reconnu sur {port_retenu} ({}). 123,45 envoye a {debit_retenu} bauds.",
            sonde.verdict
        )
    } else {
        format!(
            "123,45 envoye sur {port_retenu} a {debit_retenu} bauds, mais {}",
            sonde.detail
        )
    };

    Ok(Essai {
        message,
        verdict: sonde.verdict,
        prouve: sonde.prouve,
    })
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
    let voie = texte(CLE_AFFICHEUR_VOIE);

    Ok(ReglagesAfficheur {
        actif: texte(CLE_AFFICHEUR_ACTIF) == "oui",
        voie: if voie.is_empty() {
            afficheur::VOIE_SERIE.to_string()
        } else {
            voie
        },
        imprimante: super::impression::imprimante_retenue(connexion)?,
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
        materiel: texte(CLE_MATERIEL),
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

    fn appareil(nom: &str, famille: &str, identifiant: &str) -> Peripherique {
        Peripherique {
            nom: nom.to_string(),
            famille: famille.to_string(),
            liaison: liaison_de(identifiant),
            etat: "en service".to_string(),
            fabricant: String::new(),
            port: port_dans(nom),
            identifiant: identifiant.to_string(),
        }
    }

    /// Releve reel d une caisse : clavier integre a trois interfaces, douchette
    /// a deux, dalle tactile, et du materiel interne sans identite USB.
    fn poste() -> Vec<Peripherique> {
        vec![
            appareil(
                "Peripherique clavier IHM",
                "Keyboard",
                "HID\\VID_C0F4&PID_07C0&MI_00\\8&203712E9&0&0000",
            ),
            appareil(
                "Peripherique d entree USB",
                "HIDClass",
                "USB\\VID_C0F4&PID_07C0&MI_00\\7&38058586&0&0000",
            ),
            appareil(
                "Peripherique de controle consommateur IHM",
                "HIDClass",
                "HID\\VID_C0F4&PID_07C0&MI_01&COL01\\8&87386A4&0&0000",
            ),
            appareil(
                "Peripherique d entree USB",
                "HIDClass",
                "USB\\VID_0218&PID_0210\\2027300413413333",
            ),
            appareil(
                "Peripherique clavier IHM",
                "Keyboard",
                "HID\\VID_0218&PID_0210\\7&3C05F&0&0000",
            ),
            appareil(
                "Ecran tactile IHM",
                "HIDClass",
                "HID\\VID_222A&PID_0001&COL01\\7&A47354B&0&0000",
            ),
            appareil(
                "Moniteur Plug-and-Play generique",
                "Monitor",
                "DISPLAY\\MS_0000\\4&856DDBD&0&UID67568640",
            ),
            appareil("Port de communication (COM1)", "Ports", "ACPI\\PNP0501\\1"),
        ]
    }

    #[test]
    fn ne_retient_que_les_appareils_qui_tapent_des_touches() {
        let candidats = candidats_douchette(&poste());
        let identites: Vec<&str> = candidats
            .iter()
            .map(|candidat| candidat.materiel.as_str())
            .collect();

        // La dalle tactile et le moniteur portent une identite USB mais aucune
        // interface clavier : LCOS ne saurait pas les lire.
        assert_eq!(identites, vec!["0218:0210", "C0F4:07C0"]);
    }

    #[test]
    fn regroupe_les_interfaces_d_un_meme_appareil() {
        let candidats = candidats_douchette(&poste());

        let douchette = &candidats[0];
        let clavier_de_caisse = &candidats[1];

        assert_eq!(douchette.materiel, "0218:0210");
        assert_eq!(douchette.interfaces, 2);

        assert_eq!(clavier_de_caisse.materiel, "C0F4:07C0");
        assert_eq!(clavier_de_caisse.interfaces, 3);
    }

    #[test]
    fn nomme_le_candidat_par_son_interface_clavier() {
        let candidats = candidats_douchette(&poste());

        assert!(candidats
            .iter()
            .all(|candidat| candidat.nom == "Peripherique clavier IHM"));
    }

    #[test]
    fn un_poste_sans_clavier_usb_ne_propose_aucune_douchette() {
        let interne = vec![
            appareil("Port de communication (COM1)", "Ports", "ACPI\\PNP0501\\1"),
            appareil(
                "Intel(R) HD Graphics 3000",
                "Display",
                "PCI\\VEN_8086&DEV_0116",
            ),
        ];

        assert!(candidats_douchette(&interne).is_empty());
    }

    #[test]
    fn n_envoie_rien_sur_un_port_qu_on_ne_peut_pas_joindre() {
        assert!(port_injoignable(afficheur::SONDE_ABSENT));
        assert!(port_injoignable(afficheur::SONDE_OCCUPE));
        assert!(port_injoignable(afficheur::SONDE_INVALIDE));
        // Une ligne tenue par l appareil ferait attendre l ecriture sans fin.
        assert!(port_injoignable(afficheur::SONDE_BLOQUE));
    }

    #[test]
    fn un_port_muet_recoit_quand_meme_la_somme_d_essai() {
        // Un cable a trois fils ne porte pas DSR : l afficheur marche sans
        // qu aucune preuve ne soit possible. L essai doit avoir lieu.
        assert!(!port_injoignable(afficheur::SONDE_MUET));
        assert!(!port_injoignable(afficheur::SONDE_ALIMENTE));
        assert!(!port_injoignable(afficheur::SONDE_REPOND));
        assert!(!port_injoignable(afficheur::SONDE_ILLISIBLE));
    }
}
