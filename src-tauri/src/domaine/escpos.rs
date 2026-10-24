/*
 * Jeu de commandes ESC/POS des imprimantes thermiques 58 mm. Le spouleur
 * Windows sait deja sortir du texte brut, mais il ne sait ni couper le papier,
 * ni ouvrir le tiroir, ni passer une ligne en gras : ces ordres sont des
 * octets de controle que seul un envoi brut (datatype RAW) transporte.
 *
 * Ce module ne fait que produire les octets. L envoi au materiel est du ressort
 * de la couche commande, qui parle au spouleur de Windows.
 */

/// Initialisation : ESC @ remet l imprimante dans un etat connu, quel que soit
/// le ticket precedent.
const INITIALISER: &[u8] = &[0x1B, 0x40];

/// ESC t 0 : table de caracteres 437. Tout le logiciel ecrit en ASCII sans
/// accent, donc cette table suffit et rend le meme texte sur tout materiel.
const TABLE_CARACTERES: &[u8] = &[0x1B, 0x74, 0x00];

/// ESC R 0 : jeu international des Etats-Unis, aucune substitution de symbole.
const JEU_INTERNATIONAL: &[u8] = &[0x1B, 0x52, 0x00];

const ALIGNEMENT_GAUCHE: &[u8] = &[0x1B, 0x61, 0x00];
const ALIGNEMENT_CENTRE: &[u8] = &[0x1B, 0x61, 0x01];

const GRAS_ACTIF: &[u8] = &[0x1B, 0x45, 0x01];
const GRAS_INACTIF: &[u8] = &[0x1B, 0x45, 0x00];

/// GS ! : taille de caractere. 0x01 double la hauteur sans toucher a la
/// largeur, ce qui garde les 32 colonnes de la mise en page.
const TAILLE_DOUBLE_HAUTEUR: &[u8] = &[0x1D, 0x21, 0x01];
const TAILLE_NORMALE: &[u8] = &[0x1D, 0x21, 0x00];

/// GS V 66 0 : coupe partielle apres avoir avance le papier jusqu a la lame.
const COUPE_PARTIELLE: &[u8] = &[0x1D, 0x56, 0x42, 0x00];

/// ESC p 0 25 250 : impulsion sur la broche 2 du connecteur du tiroir-caisse.
const IMPULSION_TIROIR: &[u8] = &[0x1B, 0x70, 0x00, 0x19, 0xFA];

const RETOUR_LIGNE: u8 = 0x0A;
const SUBSTITUT: u8 = b'?';

/// Avance de fin de ticket : sans elle, les dernieres lignes restent dans le
/// mecanisme, sous la lame.
const LIGNES_AVANCE: u8 = 4;

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Premiere ligne et lignes de total en gras.
    pub gras: bool,
    /// Coupe du papier en fin de ticket.
    pub coupe: bool,
    /// Impulsion d ouverture du tiroir-caisse.
    pub tiroir: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            gras: true,
            coupe: true,
            tiroir: false,
        }
    }
}

/// Transforme le ticket deja mis en page en colonnes en un flux ESC/POS.
/// La mise en page (alignements par espaces, largeur) n est pas refaite : ce
/// qui est verifie a l ecran est exactement ce qui part sur le rouleau.
pub fn encoder(texte: &str, options: &Options) -> Vec<u8> {
    let mut flux: Vec<u8> = Vec::new();

    flux.extend_from_slice(INITIALISER);
    flux.extend_from_slice(TABLE_CARACTERES);
    flux.extend_from_slice(JEU_INTERNATIONAL);

    for (rang, ligne) in texte.lines().enumerate() {
        let nettoyee = nettoyer(ligne);

        if rang == 0 {
            poser_entete(&mut flux, &nettoyee, options);
            continue;
        }

        poser_ligne(&mut flux, &nettoyee, options);
    }

    flux.extend_from_slice(&[0x1B, 0x64, LIGNES_AVANCE]);

    if options.tiroir {
        flux.extend_from_slice(IMPULSION_TIROIR);
    }

    if options.coupe {
        flux.extend_from_slice(COUPE_PARTIELLE);
    }

    flux
}

/// Nom du commerce : centre par l imprimante, donc on retire le centrage par
/// espaces fait pour l ecran, sinon la ligne part vers la droite.
fn poser_entete(flux: &mut Vec<u8>, ligne: &[u8], options: &Options) {
    flux.extend_from_slice(ALIGNEMENT_CENTRE);
    flux.extend_from_slice(TAILLE_DOUBLE_HAUTEUR);

    if options.gras {
        flux.extend_from_slice(GRAS_ACTIF);
    }

    flux.extend_from_slice(rogner(ligne));
    flux.push(RETOUR_LIGNE);

    if options.gras {
        flux.extend_from_slice(GRAS_INACTIF);
    }

    flux.extend_from_slice(TAILLE_NORMALE);
    flux.extend_from_slice(ALIGNEMENT_GAUCHE);
}

fn poser_ligne(flux: &mut Vec<u8>, ligne: &[u8], options: &Options) {
    let accentuee = options.gras && est_un_total(ligne);

    if accentuee {
        flux.extend_from_slice(GRAS_ACTIF);
    }

    flux.extend_from_slice(ligne);
    flux.push(RETOUR_LIGNE);

    if accentuee {
        flux.extend_from_slice(GRAS_INACTIF);
    }
}

/// Les lignes de total portent le montant que le client verifie : elles sont
/// les seules du corps du ticket a passer en gras.
fn est_un_total(ligne: &[u8]) -> bool {
    ligne.starts_with(b"TOTAL") || ligne.starts_with(b"Cumul encaisse")
}

/// Un octet hors ASCII imprimable devient un point d interrogation : la table
/// 437 ne garantit rien au dela, et un octet inconnu decale toute la ligne.
fn nettoyer(ligne: &str) -> Vec<u8> {
    ligne
        .chars()
        .map(|caractere| {
            if caractere.is_ascii() && !caractere.is_ascii_control() {
                caractere as u8
            } else {
                SUBSTITUT
            }
        })
        .collect()
}

fn rogner(ligne: &[u8]) -> &[u8] {
    let debut = ligne
        .iter()
        .position(|octet| *octet != b' ')
        .unwrap_or(ligne.len());
    let fin = ligne
        .iter()
        .rposition(|octet| *octet != b' ')
        .map(|rang| rang + 1)
        .unwrap_or(debut);

    &ligne[debut..fin]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contient(flux: &[u8], motif: &[u8]) -> bool {
        flux.windows(motif.len()).any(|fenetre| fenetre == motif)
    }

    #[test]
    fn ouvre_par_une_initialisation() {
        let flux = encoder("MON COMMERCE", &Options::default());

        assert!(flux.starts_with(INITIALISER));
        assert!(contient(&flux, TABLE_CARACTERES));
    }

    #[test]
    fn termine_par_la_coupe_quand_elle_est_demandee() {
        let avec = encoder("A\nB", &Options::default());
        let sans = encoder(
            "A\nB",
            &Options {
                coupe: false,
                ..Options::default()
            },
        );

        assert!(avec.ends_with(COUPE_PARTIELLE));
        assert!(!contient(&sans, COUPE_PARTIELLE));
    }

    #[test]
    fn ouvre_le_tiroir_sur_demande_seulement() {
        let sans = encoder("A", &Options::default());
        let avec = encoder(
            "A",
            &Options {
                tiroir: true,
                ..Options::default()
            },
        );

        assert!(!contient(&sans, IMPULSION_TIROIR));
        assert!(contient(&avec, IMPULSION_TIROIR));
    }

    #[test]
    fn passe_la_ligne_de_total_en_gras() {
        let flux = encoder("MON COMMERCE\nTOTAL EUR  3,90", &Options::default());
        let gras = flux
            .windows(GRAS_ACTIF.len())
            .filter(|fenetre| *fenetre == GRAS_ACTIF)
            .count();

        // Une fois pour l entete, une fois pour le total.
        assert_eq!(gras, 2);
    }

    #[test]
    fn laisse_le_corps_du_ticket_intact() {
        let flux = encoder("MON COMMERCE\n  2 x 1,50            3,00", &Options::default());

        assert!(contient(&flux, b"  2 x 1,50            3,00"));
    }

    #[test]
    fn rogne_le_centrage_par_espaces_de_l_entete() {
        let flux = encoder("      MON COMMERCE", &Options::default());

        assert!(contient(&flux, b"MON COMMERCE"));
        assert!(!contient(&flux, b"      MON COMMERCE"));
    }

    #[test]
    fn remplace_un_caractere_hors_table() {
        let flux = encoder("Cafe\nPrix eleve\u{20ac}", &Options::default());

        assert!(contient(&flux, b"Prix eleve?"));
    }

    #[test]
    fn sans_gras_aucune_commande_de_gras() {
        let flux = encoder(
            "MON COMMERCE\nTOTAL EUR  3,90",
            &Options {
                gras: false,
                ..Options::default()
            },
        );

        assert!(!contient(&flux, GRAS_ACTIF));
    }
}
