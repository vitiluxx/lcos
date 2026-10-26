/*
 * Afficheur client et inventaire du materiel.
 *
 * L afficheur client est le petit ecran tourne vers le client. Il ne porte
 * qu une somme, poussee a chaque changement du total. Deux regles tiennent
 * tout ce fichier :
 *   - une panne de l afficheur n arrete jamais une vente : l envoi echoue en
 *     silence, une seule fois signale, et la caisse continue ;
 *   - rien n est envoye tant que le gerant n a pas allume l afficheur.
 *
 * L inventaire liste ce que Windows reconnait sur la machine. Il sert a
 * trouver le port de l afficheur sans ouvrir le gestionnaire de peripheriques.
 */
(function (global) {
  "use strict";

  const element = global.format.element;

  // Deux envois rapproches sur un port serie a 9600 bauds se marchent dessus :
  // on ne pousse que la derniere somme d une rafale de frappes.
  const DELAI_ENVOI_MS = 120;

  let reglages = null;
  let inventaire = null;
  let minuterie = null;
  let dernierEnvoi = null;
  let enPanne = false;

  function badge(identifiant, texte, nature) {
    const noeud = element(identifiant);

    noeud.textContent = texte;
    noeud.className = "badge badge--" + nature;
  }

  /*
   * ------------------------------ afficheur -------------------------------
   */

  function remplirPorts(ports) {
    const liste = element("afficheur_port");
    const retenu = (reglages && reglages.port) || liste.value;
    const connus = ports.slice();

    // Un port deja retenu reste proposable meme si l appareil est debranche :
    // sinon le reglage disparaitrait de l ecran a la premiere coupure.
    if (retenu && connus.indexOf(retenu) === -1) {
      connus.push(retenu);
    }

    global.format.vider(liste);

    if (!connus.length) {
      liste.appendChild(new Option("Aucun port serie detecte", ""));
      element("afficheur_port_aide").textContent =
        "aucun port serie : releve le materiel plus bas, l afficheur doit y figurer avec un COM";
      return;
    }

    connus.sort();
    connus.forEach(function (port) {
      liste.appendChild(new Option(port, port));
    });

    liste.value = retenu;

    // Aucun port retenu : le premier detecte est propose, sinon la liste
    // paraitrait vide alors qu elle porte des choix.
    if (!liste.value) {
      liste.selectedIndex = 0;
    }

    element("afficheur_port_aide").textContent =
      connus.length + " port(s) serie detecte(s) : " + connus.join(", ");
  }

  function appliquerAfficheur(courant) {
    reglages = courant;
    enPanne = false;

    element("afficheur_actif").checked = courant.actif;
    element("afficheur_debit").value = String(courant.debit);
    element("afficheur_protocole").value = courant.protocole;
    element("afficheur_colonnes").value = String(courant.colonnes);

    remplirPorts(inventaire ? inventaire.ports_serie : []);

    if (!courant.actif) {
      badge("badge_afficheur", "eteint", "attente");
      return;
    }

    badge("badge_afficheur", courant.port || "port manquant", "succes");

    if (courant.dernier_envoi) {
      element("afficheur_verdict").textContent = "dernier envoi : " + courant.dernier_envoi;
    }
  }

  async function rafraichir() {
    try {
      appliquerAfficheur(await global.pont.appeler("recuperer_reglages_afficheur"));
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function enregistrer() {
    try {
      appliquerAfficheur(
        await global.pont.appeler("enregistrer_reglages_afficheur", {
          jeton: global.acces.exiger(),
          actif: element("afficheur_actif").checked,
          port: element("afficheur_port").value,
          debit: global.format.entier(element("afficheur_debit").value, 9600),
          protocole: element("afficheur_protocole").value,
          colonnes: global.format.entier(element("afficheur_colonnes").value, 20)
        })
      );

      global.notifier.succes(
        reglages.actif
          ? "Afficheur client allume sur " + reglages.port + "."
          : "Afficheur client eteint."
      );
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function tester() {
    try {
      const verdict = await global.pont.appeler("tester_afficheur", {
        jeton: global.acces.exiger(),
        port: element("afficheur_port").value,
        debit: global.format.entier(element("afficheur_debit").value, 9600),
        protocole: element("afficheur_protocole").value,
        colonnes: global.format.entier(element("afficheur_colonnes").value, 20)
      });

      element("afficheur_verdict").textContent = verdict;
      global.notifier.succes(verdict);
      enPanne = false;
    } catch (incident) {
      element("afficheur_verdict").textContent = incident.message || String(incident);
      global.notifier.incident(incident);
    }
  }

  async function effacer() {
    try {
      const envoye = await global.pont.appeler("effacer_afficheur_client");

      element("afficheur_verdict").textContent = envoye
        ? "ecran rendu a blanc"
        : "afficheur eteint : rien a effacer";
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  // Appelee par la caisse a chaque changement du total. Volontairement muette :
  // le caissier encaisse, il n a pas a lire des messages d afficheur.
  function poserSomme(centimes) {
    if (!reglages || !reglages.actif || enPanne) {
      return;
    }

    const valeur = Number(centimes) || 0;

    if (valeur === dernierEnvoi) {
      return;
    }

    dernierEnvoi = valeur;

    if (minuterie !== null) {
      clearTimeout(minuterie);
    }

    minuterie = setTimeout(async function () {
      minuterie = null;

      try {
        await global.pont.appeler("afficher_somme_client", {
          centimes: valeur,
          decimales: global.format.avecDecimales() ? 2 : 0
        });
      } catch (incident) {
        // Une seule alerte, puis silence : un port mort ne doit pas noyer
        // l ecran de notifications a chaque frappe.
        enPanne = true;
        badge("badge_afficheur", "en defaut", "alerte");
        element("afficheur_verdict").textContent = incident.message || String(incident);
        global.notifier.alerte(
          "Afficheur client muet : " + (incident.message || String(incident))
        );
      }
    }, DELAI_ENVOI_MS);
  }

  /*
   * ------------------------------ inventaire ------------------------------
   */

  function remplirFamilles() {
    const liste = element("inventaire_famille");
    const choisie = liste.value;
    const familles = [];

    inventaire.peripheriques.forEach(function (appareil) {
      if (familles.indexOf(appareil.famille) === -1) {
        familles.push(appareil.famille);
      }
    });

    familles.sort();
    global.format.vider(liste);
    liste.appendChild(new Option("Toutes les familles", ""));

    familles.forEach(function (famille) {
      liste.appendChild(new Option(famille, famille));
    });

    liste.value = choisie;
  }

  function retenus() {
    const recherche = element("inventaire_recherche").value.trim().toLowerCase();
    const famille = element("inventaire_famille").value;

    return inventaire.peripheriques.filter(function (appareil) {
      if (famille && appareil.famille !== famille) {
        return false;
      }

      if (!recherche) {
        return true;
      }

      return (
        appareil.nom.toLowerCase().indexOf(recherche) !== -1 ||
        appareil.famille.toLowerCase().indexOf(recherche) !== -1 ||
        appareil.liaison.toLowerCase().indexOf(recherche) !== -1 ||
        appareil.port.toLowerCase().indexOf(recherche) !== -1 ||
        appareil.fabricant.toLowerCase().indexOf(recherche) !== -1
      );
    });
  }

  function afficherInventaire() {
    const corps = element("inventaire_liste");

    global.format.vider(corps);

    if (!inventaire) {
      corps.appendChild(global.format.ligneVide(6, "Materiel non releve."));
      return;
    }

    const liste = retenus();

    element("inventaire_resume").textContent =
      liste.length +
      " appareil(s) affiche(s) sur " +
      inventaire.nombre +
      " detecte(s), releve du " +
      inventaire.releve_le +
      (inventaire.tronque ? " (liste ecourtee)" : "");

    if (!liste.length) {
      corps.appendChild(global.format.ligneVide(6, "Aucun appareil pour ce filtre."));
      return;
    }

    liste.forEach(function (appareil) {
      const ligne = global.format.creer("tr");

      ligne.appendChild(global.format.cellule(appareil.nom));
      ligne.appendChild(global.format.cellule(appareil.famille));
      ligne.appendChild(global.format.cellule(appareil.liaison));
      ligne.appendChild(global.format.cellule(appareil.port || "--"));

      const etat = global.format.creer("td");

      etat.appendChild(
        global.format.creer(
          "span",
          appareil.etat === "en service" ? "badge badge--succes" : "badge badge--attente",
          appareil.etat
        )
      );
      ligne.appendChild(etat);
      ligne.appendChild(global.format.cellule(appareil.fabricant || "--"));

      corps.appendChild(ligne);
    });
  }

  // Le service de Windows qui decrit le materiel refuse parfois la premiere
  // interrogation juste apres le demarrage, le temps qu il s eveille : un
  // second essai vaut mieux qu un message d echec a la premiere seconde.
  async function interroger() {
    try {
      return await global.pont.appeler("inventorier_peripheriques", {
        jeton: global.acces.exiger()
      });
    } catch (premier) {
      await new Promise(function (suite) {
        setTimeout(suite, 1200);
      });

      return global.pont.appeler("inventorier_peripheriques", {
        jeton: global.acces.exiger()
      });
    }
  }

  async function relever() {
    const bouton = element("inventaire_relever");

    bouton.disabled = true;
    badge("badge_inventaire", "lecture en cours", "attente");

    try {
      inventaire = await interroger();

      badge("badge_inventaire", inventaire.nombre + " appareils", "succes");
      remplirFamilles();
      afficherInventaire();
      remplirPorts(inventaire.ports_serie);
    } catch (incident) {
      badge("badge_inventaire", "lecture impossible", "alerte");
      global.notifier.incident(incident);
    } finally {
      bouton.disabled = false;
    }
  }

  function initialiser() {
    element("afficheur_enregistrer").addEventListener("click", enregistrer);
    element("afficheur_tester").addEventListener("click", tester);
    element("afficheur_effacer").addEventListener("click", effacer);
    element("inventaire_relever").addEventListener("click", relever);
    element("inventaire_recherche").addEventListener("input", afficherInventaire);
    element("inventaire_famille").addEventListener("change", afficherInventaire);

    afficherInventaire();
  }

  global.vueAfficheur = {
    initialiser: initialiser,
    rafraichir: rafraichir,
    poserSomme: poserSomme,
    actif: function () {
      return Boolean(reglages && reglages.actif);
    }
  };
})(window);
