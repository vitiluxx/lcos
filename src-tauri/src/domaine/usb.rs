/*
 * Identite materielle d un appareil USB.
 *
 * Windows nomme un appareil librement : deux douchettes du meme lot peuvent
 * s appeler `Peripherique d entree USB` comme la moitie du poste. Ce qui ne
 * change jamais, c est le couple fabricant / produit grave dans l appareil et
 * repris dans son identifiant systeme :
 *
 *     USB\VID_30FA&PID_0302\6&186AFA3&0&4
 *          ^^^^     ^^^^
 *
 * D ou la forme courte `30FA:0302`, retenue comme signature materielle de la
 * douchette. Elle survit a un changement de port, a une reinstallation du
 * pilote et a un redemarrage, ce qu une mesure de cadence de frappe ne fait
 * pas.
 */
use crate::erreur::{ErreurApplication, Resultat};

/// Longueur d un identifiant `VID_`/`PID_` : quatre chiffres hexadecimaux.
const LONGUEUR_CODE: usize = 4;

/// `VID:PID` lu dans un identifiant systeme de Windows. Vide quand l appareil
/// n est pas sur un bus qui porte ces codes : une carte PCI, un port de carte
/// mere ou un peripherique logiciel n en ont pas.
pub fn identite(identifiant: &str) -> Option<String> {
    let majuscules = identifiant.to_ascii_uppercase();
    let fabricant = code(&majuscules, "VID_")?;
    let produit = code(&majuscules, "PID_")?;

    Some(format!("{fabricant}:{produit}"))
}

fn code(identifiant: &str, etiquette: &str) -> Option<String> {
    let debut = identifiant.find(etiquette)? + etiquette.len();

    let valeur: String = identifiant[debut..]
        .chars()
        .take(LONGUEUR_CODE)
        .filter(|caractere| caractere.is_ascii_hexdigit())
        .collect();

    if valeur.len() == LONGUEUR_CODE {
        return Some(valeur);
    }

    None
}

/// Une identite retenue en reglage, ou rien. Le vide est permis : il dit que
/// la douchette n a pas encore ete designee, et c est un etat normal.
pub fn valider_identite(materiel: &str) -> Resultat<String> {
    let nettoye = materiel.trim().to_ascii_uppercase();

    if nettoye.is_empty() {
        return Ok(String::new());
    }

    if let Some((fabricant, produit)) = nettoye.split_once(':') {
        if est_un_code(fabricant) && est_un_code(produit) {
            return Ok(nettoye);
        }
    }

    Err(ErreurApplication::Validation(format!(
        "identite materielle attendue sous la forme VID:PID, par exemple 30FA:0302 : {materiel}"
    )))
}

fn est_un_code(valeur: &str) -> bool {
    valeur.len() == LONGUEUR_CODE && valeur.chars().all(|caractere| caractere.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_l_identite_dans_un_identifiant_usb() {
        assert_eq!(
            identite("USB\\VID_30FA&PID_0302\\6&186AFA3&0&4").unwrap(),
            "30FA:0302"
        );
        assert_eq!(
            identite("HID\\VID_c0f4&PID_07c0&MI_00\\8&203712E9&0&0000").unwrap(),
            "C0F4:07C0"
        );
    }

    #[test]
    fn n_invente_pas_d_identite_pour_le_materiel_interne() {
        assert!(identite("PCI\\VEN_8086&DEV_0116").is_none());
        assert!(identite("ACPI\\PNP0501\\1").is_none());
        assert!(identite("DISPLAY\\MS_0000\\4&856DDBD&0&UID67568640").is_none());
        assert!(identite("").is_none());
    }

    #[test]
    fn ecarte_un_code_tronque() {
        assert!(identite("USB\\VID_30F&PID_0302").is_none());
        assert!(identite("USB\\VID_30FA&PID_03").is_none());
        // `VID_` sans `PID_` ne designe rien d exploitable.
        assert!(identite("USB\\VID_30FA\\6&1").is_none());
    }

    #[test]
    fn accepte_une_identite_vide_comme_douchette_non_designee() {
        assert_eq!(valider_identite("").unwrap(), "");
        assert_eq!(valider_identite("   ").unwrap(), "");
    }

    #[test]
    fn normalise_une_identite_retenue() {
        assert_eq!(valider_identite(" 30fa:0302 ").unwrap(), "30FA:0302");
    }

    #[test]
    fn refuse_ce_qui_n_est_pas_une_identite() {
        assert!(valider_identite("30FA").is_err());
        assert!(valider_identite("30FA:030").is_err());
        assert!(valider_identite("30FA:0302:0001").is_err());
        assert!(valider_identite("ZZZZ:0302").is_err());
        assert!(valider_identite("../../etc/passwd").is_err());
    }
}
