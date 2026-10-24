/*
 * Outil de l editeur : fabrique les licences de LCOS.
 *
 * Il ne part jamais chez le client. Il vit sur la machine de l editeur, avec
 * la cle privee, et sert a emettre une licence pour un poste installe depuis
 * des mois, hors ligne, sans retoucher le logiciel livre.
 *
 *   cargo run --bin lcos_licence -- cles
 *   cargo run --bin lcos_licence -- generer --cle-privee @cle_privee.txt \
 *       --numero 0001 --titulaire "Boutique Chez Ali" --jours 365
 *   cargo run --bin lcos_licence -- verifier @licence.txt
 *
 * Une valeur prefixee par @ est lue dans le fichier qui suit : la cle privee
 * ne passe pas par la ligne de commande, donc pas par l historique du shell.
 */
use std::collections::HashMap;
use std::process::ExitCode;

use lcos_lib::domaine::horodatage;
use lcos_lib::domaine::licence::{self, Charge};

const AIDE: &str = "\
Outil de licences LCOS

  lcos_licence cles
      Fabrique une paire de cles Ed25519. La publique se colle dans
      src/domaine/licence.rs (CLE_PUBLIQUE_HEX), la privee reste chez
      l editeur, hors du depot.

  lcos_licence generer --cle-privee @chemin --numero N --titulaire \"Nom\"
                       [--jours N | --perpetuelle] [--poste CODE]
                       [--emise-le AAAA-MM-JJ] [--sortie chemin]
      Emet une licence signee. Sans --poste, la licence vaut pour tout poste ;
      avec, elle ne vaut que pour l empreinte affichee par la caisse.

  lcos_licence verifier [--cle-publique HEX] <licence|@chemin>
      Relit une licence avec le meme code que la caisse.
";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    let Some(commande) = arguments.first().cloned() else {
        print!("{AIDE}");

        return ExitCode::FAILURE;
    };

    let reste = &arguments[1..];

    let resultat = match commande.as_str() {
        "cles" => cles(),
        "generer" => generer(reste),
        "verifier" => verifier(reste),
        "aide" | "--aide" | "-h" | "--help" => {
            print!("{AIDE}");

            Ok(())
        }
        autre => Err(format!(
            "commande inconnue : {autre}\n\n{AIDE}"
        )),
    };

    match resultat {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("echec : {message}");

            ExitCode::FAILURE
        }
    }
}

fn cles() -> Result<(), String> {
    let signataire = ed25519_dalek::SigningKey::generate(&mut rand_core::OsRng);
    let privee = licence::hexadecimal(&signataire.to_bytes());
    let publique = licence::hexadecimal(signataire.verifying_key().as_bytes());

    println!("cle privee  (a garder secrete, hors du depot) :");
    println!("{privee}");
    println!();
    println!("cle publique (a coller dans CLE_PUBLIQUE_HEX) :");
    println!("{publique}");

    Ok(())
}

fn generer(arguments: &[String]) -> Result<(), String> {
    let options = decouper(arguments)?;

    let cle_privee = exiger(&options, "cle-privee")?;
    let numero = exiger(&options, "numero")?;
    let titulaire = exiger(&options, "titulaire")?;
    let emise_le = options
        .get("emise-le")
        .cloned()
        .unwrap_or_else(horodatage::aujourd_hui);

    if horodatage::jours_depuis_epoque(&emise_le).is_none() {
        return Err(format!("date d emission illisible : {emise_le}"));
    }

    let perpetuelle = options.contains_key("perpetuelle");
    let jours = options.get("jours").cloned();

    if perpetuelle && jours.is_some() {
        return Err("--jours et --perpetuelle s excluent".to_string());
    }

    let expire_le = match (perpetuelle, jours) {
        (true, _) => None,
        (false, Some(valeur)) => {
            let duree: i64 = valeur
                .trim()
                .parse()
                .map_err(|_| format!("--jours : nombre attendu, recu {valeur}"))?;

            if duree < 1 {
                return Err("--jours : au moins 1".to_string());
            }

            let debut = horodatage::jours_depuis_epoque(&emise_le)
                .ok_or_else(|| "date d emission illisible".to_string())?;

            Some(horodatage::date_depuis_epoque(debut + duree))
        }
        (false, None) => {
            return Err("duree manquante : --jours N ou --perpetuelle".to_string())
        }
    };

    let charge = Charge {
        numero,
        titulaire,
        emise_le,
        expire_le,
        poste: options
            .get("poste")
            .map(|brut| licence::formater_poste(brut))
            .filter(|poste| !poste.is_empty()),
    };

    let texte = licence::signer(&charge, &cle_privee).map_err(|erreur| erreur.to_string())?;

    // Relecture immediate avec la cle publique deduite : une licence emise est
    // verifiee avant d etre remise au client.
    let publique = cle_publique_de(&cle_privee)?;

    licence::verifier_avec(&texte, &publique).map_err(|erreur| erreur.to_string())?;

    println!("licence pour {} (numero {})", charge.titulaire, charge.numero);
    println!(
        "emise le {} - {}",
        charge.emise_le,
        charge
            .expire_le
            .as_deref()
            .map(|fin| format!("valable jusqu au {fin}"))
            .unwrap_or_else(|| "perpetuelle".to_string())
    );
    println!(
        "poste : {}",
        charge.poste.as_deref().unwrap_or("tout poste")
    );
    println!();
    println!("{texte}");

    if let Some(chemin) = options.get("sortie") {
        std::fs::write(chemin, format!("{texte}\n"))
            .map_err(|erreur| format!("ecriture de {chemin} impossible : {erreur}"))?;

        println!();
        println!("ecrit dans {chemin}");
    }

    Ok(())
}

fn verifier(arguments: &[String]) -> Result<(), String> {
    let options = decouper(arguments)?;
    let texte = options
        .get("")
        .cloned()
        .ok_or_else(|| "licence a verifier manquante".to_string())?;

    let publique = options
        .get("cle-publique")
        .cloned()
        .unwrap_or_else(|| licence::CLE_PUBLIQUE_HEX.to_string());

    let charge =
        licence::verifier_avec(&texte, &publique).map_err(|erreur| erreur.to_string())?;

    println!("licence valide");
    println!("  numero     : {}", charge.numero);
    println!("  titulaire  : {}", charge.titulaire);
    println!("  emise le   : {}", charge.emise_le);
    println!(
        "  validite   : {}",
        charge
            .expire_le
            .as_deref()
            .map(|fin| format!("jusqu au {fin}"))
            .unwrap_or_else(|| "perpetuelle".to_string())
    );
    println!(
        "  poste      : {}",
        charge.poste.as_deref().unwrap_or("tout poste")
    );

    Ok(())
}

fn cle_publique_de(cle_privee_hex: &str) -> Result<String, String> {
    let mut octets = [0u8; 32];
    let nettoye: String = cle_privee_hex
        .chars()
        .filter(|caractere| !caractere.is_whitespace())
        .collect();

    if nettoye.len() != 64 {
        return Err("cle privee : 64 caracteres hexadecimaux attendus".to_string());
    }

    for (rang, paire) in nettoye.as_bytes().chunks(2).enumerate() {
        let texte = std::str::from_utf8(paire).unwrap_or("zz");

        octets[rang] = u8::from_str_radix(texte, 16)
            .map_err(|_| "cle privee : hexadecimal invalide".to_string())?;
    }

    let signataire = ed25519_dalek::SigningKey::from_bytes(&octets);

    Ok(licence::hexadecimal(signataire.verifying_key().as_bytes()))
}

/// Decoupe `--option valeur` et retient sous la cle vide le premier argument
/// libre. Une valeur `@chemin` est remplacee par le contenu du fichier.
fn decouper(arguments: &[String]) -> Result<HashMap<String, String>, String> {
    let mut options: HashMap<String, String> = HashMap::new();
    let mut rang = 0;

    while rang < arguments.len() {
        let courant = &arguments[rang];

        if let Some(nom) = courant.strip_prefix("--") {
            let suivant = arguments.get(rang + 1);

            match suivant {
                Some(valeur) if !valeur.starts_with("--") => {
                    options.insert(nom.to_string(), lire_valeur(valeur)?);
                    rang += 2;
                }
                _ => {
                    options.insert(nom.to_string(), String::new());
                    rang += 1;
                }
            }

            continue;
        }

        options.entry(String::new()).or_insert(lire_valeur(courant)?);
        rang += 1;
    }

    Ok(options)
}

fn lire_valeur(brut: &str) -> Result<String, String> {
    match brut.strip_prefix('@') {
        Some(chemin) => std::fs::read_to_string(chemin)
            .map(|contenu| contenu.trim().to_string())
            .map_err(|erreur| format!("lecture de {chemin} impossible : {erreur}")),
        None => Ok(brut.to_string()),
    }
}

fn exiger(options: &HashMap<String, String>, nom: &str) -> Result<String, String> {
    match options.get(nom) {
        Some(valeur) if !valeur.trim().is_empty() => Ok(valeur.trim().to_string()),
        _ => Err(format!("--{nom} manquant")),
    }
}
