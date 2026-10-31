/*
 * Ecrans de la caisse.
 *
 * Une caisse porte deux ecrans qui n ont rien a voir l un avec l autre :
 *   - l ecran de face, celui du caissier, un vrai moniteur que Windows gere
 *     comme n importe quel ecran d ordinateur ;
 *   - l ecran arriere, tourne vers le client, qui n affiche que des chiffres.
 *     Celui-la n est presque jamais un moniteur : c est un afficheur serie,
 *     traite par `domaine::afficheur`, et Windows ne le voit pas du tout.
 *
 * Ce module ne repond donc qu a une question, mais il y repond nettement :
 * **combien de vrais moniteurs sont branches, et lequel est celui de face.**
 * Le moniteur principal est celui du caissier : c est la que Windows ouvre les
 * fenetres. Deux moniteurs releves ici valent avertissement : la caisse a un
 * second ecran d ordinateur, et l afficheur client n est pas celui-la.
 *
 * `Win32_PnPEntity` de l inventaire dit qu un moniteur existe ; il ne dit ni
 * sa resolution, ni son rang. Ces deux-la viennent de `Screen::AllScreens`,
 * et le modele grave dans l EDID de `WmiMonitorID`.
 */
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Ecran {
    /// Rang d enumeration de Windows, a partir de 1.
    pub rang: i64,
    /// L ecran ou Windows ouvre les fenetres : celui du caissier.
    pub principal: bool,
    pub largeur: i64,
    pub hauteur: i64,
    /// Modele grave dans l EDID de la dalle. Souvent vide : une dalle de
    /// caisse integree n en declare aucun, et Windows la nomme alors
    /// `Moniteur Plug-and-Play generique`.
    pub modele: String,
}

/// Consigne PowerShell qui releve les ecrans. Une ligne par ecran :
/// `ECRAN<tab>rang<tab>principal<tab>largeur<tab>hauteur<tab>modele`.
///
/// Le modele n est repris que lorsqu il n y a **qu un** ecran et **qu un**
/// EDID actif. Au-dela, rien ne permet de rattacher un EDID a un rang sans
/// interroger le pilote d affichage : plutot que d associer au hasard, le
/// champ reste vide et le nom Windows de l inventaire fait foi.
///
/// Aucun guillemet double ici : la consigne traverse la ligne de commande de
/// `powershell.exe`, qui les mange. Les colonnes sont assemblees par
/// `-join [char]9`.
pub const CONSIGNE: &str = "[Console]::OutputEncoding = [Text.Encoding]::UTF8; \
     Add-Type -AssemblyName System.Windows.Forms; \
     $modeles=@(); \
     try{ $modeles=@(Get-CimInstance -Namespace root\\wmi -ClassName WmiMonitorID -ErrorAction Stop | \
     Where-Object{ $_.Active } | \
     ForEach-Object{ -join ($_.UserFriendlyName | Where-Object{ $_ -gt 0 } | ForEach-Object{ [char]$_ }) } | \
     Where-Object{ $_ }) }catch{}; \
     $ecrans=@([System.Windows.Forms.Screen]::AllScreens); \
     $rang=0; \
     foreach($ecran in $ecrans){ \
     $rang+=1; \
     $modele=''; \
     if($ecrans.Count -eq 1 -and $modeles.Count -eq 1){ $modele=$modeles[0] }; \
     ('ECRAN',$rang,$ecran.Primary,$ecran.Bounds.Width,$ecran.Bounds.Height,$modele) -join [char]9 }";

/// Ecrans tires de la sortie de la consigne. Une ligne abimee est ecartee :
/// un releve partiel vaut mieux qu un ecran de parametrage en erreur.
pub fn lire(sortie: &str) -> Vec<Ecran> {
    sortie.lines().filter_map(analyser).collect()
}

fn analyser(ligne: &str) -> Option<Ecran> {
    let champs: Vec<&str> = ligne.trim_end_matches('\r').split('\t').collect();

    if champs.len() < 5 || champs[0].trim() != "ECRAN" {
        return None;
    }

    Some(Ecran {
        rang: champs[1].trim().parse().ok()?,
        principal: champs[2].trim().eq_ignore_ascii_case("true"),
        largeur: champs[3].trim().parse().unwrap_or(0),
        hauteur: champs[4].trim().parse().unwrap_or(0),
        modele: champs.get(5).unwrap_or(&"").trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_l_ecran_de_face_d_une_caisse_a_un_seul_moniteur() {
        let ecrans = lire("ECRAN\t1\tTrue\t1366\t768\t");

        assert_eq!(ecrans.len(), 1);
        assert_eq!(ecrans[0].rang, 1);
        assert!(ecrans[0].principal);
        assert_eq!(ecrans[0].largeur, 1366);
        assert_eq!(ecrans[0].hauteur, 768);
        assert_eq!(ecrans[0].modele, "");
    }

    #[test]
    fn reprend_le_modele_grave_dans_l_edid() {
        let ecrans = lire("ECRAN\t1\tTrue\t1920\t1080\tPOS-15T");

        assert_eq!(ecrans[0].modele, "POS-15T");
    }

    #[test]
    fn distingue_l_ecran_de_face_du_second_moniteur() {
        let ecrans = lire("ECRAN\t1\tTrue\t1366\t768\t\r\nECRAN\t2\tFalse\t800\t600\t\r\n");

        assert_eq!(ecrans.len(), 2);
        assert!(ecrans[0].principal);
        assert!(!ecrans[1].principal);
        assert_eq!(ecrans[1].rang, 2);
    }

    #[test]
    fn ecarte_les_lignes_qui_ne_sont_pas_des_ecrans() {
        let ecrans = lire(
            "Add-Type : bruit\nECRAN\t1\tTrue\t1366\t768\t\nECRAN\tsans_rang\tTrue\t1\t1\t\nECRAN\t3\n",
        );

        assert_eq!(ecrans.len(), 1);
        assert_eq!(ecrans[0].rang, 1);
    }

    #[test]
    fn un_releve_vide_ne_rend_aucun_ecran() {
        assert!(lire("").is_empty());
    }

    #[test]
    fn la_consigne_ne_nomme_qu_une_dalle_quand_la_correspondance_est_certaine() {
        assert!(CONSIGNE.contains("$ecrans.Count -eq 1 -and $modeles.Count -eq 1"));
    }

    #[test]
    fn un_edid_illisible_n_emporte_pas_le_releve_des_ecrans() {
        // `WmiMonitorID` est refuse par certains pilotes d affichage. La
        // consigne doit rattraper ce refus, sinon le rang et la resolution des
        // ecrans seraient perdus avec le modele.
        assert!(CONSIGNE.contains("}catch{}"));
        assert!(CONSIGNE.contains("-ErrorAction Stop"));
    }

    #[test]
    fn la_consigne_ne_porte_aucun_guillemet_double() {
        // `powershell.exe` mange les guillemets doubles de sa ligne de
        // commande : les colonnes sont assemblees sans en utiliser un.
        assert!(!CONSIGNE.contains('"'));
        assert!(CONSIGNE.contains("-join [char]9"));
    }
}
