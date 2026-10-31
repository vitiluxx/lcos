(function (global) {
  "use strict";

  let reglagesCourants = null;

  // Appareils a clavier releves par l inventaire. Vides tant que le gerant n a
  // pas releve le materiel : la douchette est alors reconnue a sa frappe seule,
  // comme avant, et son identite reste a designer.
  let douchettes = [];

  function element(identifiant) {
    return document.getElementById(identifiant);
  }

  function horodatageLisible(valeur) {
    if (!valeur) {
      return "jamais";
    }

    return valeur.replace(" ", " a ") + " UTC";
  }

  function appliquerReglages(reglages) {
    reglagesCourants = reglages;
    global.scanner.appliquerReglages(reglages);

    const badge = element("badge_scanner");

    if (reglages.detecte) {
      badge.textContent = "detecte";
      badge.className = "badge badge--succes";
    } else {
      badge.textContent = "en attente";
      badge.className = "badge badge--attente";
    }

    element("valeur_signature").textContent = reglages.signature || "--";
    element("valeur_materiel").textContent = reglages.materiel || "--";
    element("valeur_derniere_detection").textContent = horodatageLisible(reglages.derniere_detection);
    element("valeur_cadence_requise").textContent = reglages.delai_max_entre_touches + " ms";
    element("valeur_longueur_requise").textContent = reglages.longueur_minimale + " caracteres";

    proposerDouchettes(douchettes);
  }

  /*
   * ---------------------------- appareil designe ----------------------------
   *
   * La frappe dit qu une douchette a scanne ; elle ne dit pas laquelle. Seule
   * l identite USB le dit, et elle ne peut venir que de l inventaire : le
   * navigateur ne connait pas l appareil derriere une touche.
   */

  function proposerDouchettes(liste) {
    douchettes = liste || [];

    const choix = element("scanner_materiel");
    const retenu = (reglagesCourants && reglagesCourants.materiel) || "";

    global.format.vider(choix);
    choix.appendChild(new Option("Aucun appareil designe", ""));

    douchettes.forEach(function (candidat) {
      choix.appendChild(
        new Option(
          candidat.materiel + " - " + candidat.nom +
            " (" + candidat.interfaces + " interface(s))",
          candidat.materiel
        )
      );
    });

    // Un appareil retenu puis debranche reste proposable : sinon le reglage
    // disparaitrait de l ecran a la premiere coupure.
    const connu = douchettes.some(function (candidat) {
      return candidat.materiel === retenu;
    });

    if (retenu && !connu) {
      choix.appendChild(new Option(retenu + " - appareil absent", retenu));
    }

    choix.value = retenu;

    element("scanner_materiel_aide").textContent = douchettes.length
      ? douchettes.length + " appareil(s) a clavier detecte(s), le plus probable en tete :" +
        " une douchette expose moins d interfaces qu un clavier de caisse."
      : "aucun appareil a clavier detecte : releve le materiel, plus bas dans cet ecran.";
  }

  async function designer() {
    try {
      const reglages = await global.pont.appeler("designer_douchette", {
        jeton: global.acces.exiger(),
        materiel: element("scanner_materiel").value
      });

      appliquerReglages(reglages);

      global.notifier.succes(
        reglages.materiel
          ? "Douchette retenue : " + reglages.materiel + "."
          : "Aucun appareil retenu comme douchette."
      );
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function afficherMesures(mesures) {
    element("valeur_caracteres").textContent = String(mesures.caracteres);
    element("valeur_cadence").textContent = mesures.ecart_moyen_ms + " ms";
    element("valeur_cadence_max").textContent = mesures.ecart_max_ms + " ms";
    element("valeur_duree").textContent = mesures.duree_ms + " ms";
  }

  function afficherVerdict(texte, nature) {
    const zone = element("verdict_scan");

    zone.textContent = texte;
    zone.className = "zone_verdict zone_verdict--" + nature;
    zone.hidden = false;
  }

  async function analyser(code) {
    try {
      const analyse = await global.pont.appeler("analyser_code_barres", { code: code });

      element("valeur_code").textContent = analyse.code;
      element("valeur_symbologie").textContent = analyse.symbologie;

      if (!analyse.somme_controle_verifiable) {
        element("valeur_controle").textContent = "non verifiable";
        afficherVerdict("Code lu. Symbologie sans cle de controle : " + analyse.symbologie + ".", "neutre");
        return;
      }

      if (analyse.somme_controle_valide) {
        element("valeur_controle").textContent = "valide";
        afficherVerdict("Code " + analyse.symbologie + " valide, cle de controle verifiee.", "succes");
      } else {
        element("valeur_controle").textContent = "invalide";
        afficherVerdict("Cle de controle " + analyse.symbologie + " incorrecte : lecture partielle ou etiquette abimee.", "alerte");
      }
    } catch (incident) {
      element("valeur_code").textContent = code;
      element("valeur_symbologie").textContent = "--";
      element("valeur_controle").textContent = "--";
      afficherVerdict("Analyse refusee : " + incident.message, "alerte");
    }
  }

  async function memoriserDetection(charge) {
    if (reglagesCourants && reglagesCourants.detecte) {
      return;
    }

    try {
      const reglages = await global.pont.appeler("enregistrer_detection_scanner", {
        signature: global.scanner.signature(charge),
        // Un seul appareil a clavier releve : le scan ne peut venir que de
        // lui. Au-dela, l identite reste au gerant, qui seul sait lequel de
        // ses claviers est la douchette.
        materiel: douchettes.length === 1 ? douchettes[0].materiel : null
      });

      appliquerReglages(reglages);
    } catch (incident) {
      console.error("memorisation de la detection impossible", incident);
    }
  }

  function surScan(charge) {
    afficherMesures(charge.mesures);
    element("valeur_suffixe").textContent = charge.suffixe;

    analyser(charge.code);
    memoriserDetection(charge);
  }

  function surRejet(charge) {
    afficherMesures(charge.mesures);
    element("valeur_suffixe").textContent = charge.suffixe;
    element("valeur_code").textContent = charge.code;
    element("valeur_symbologie").textContent = "--";
    element("valeur_controle").textContent = "--";

    const causes = [];

    if (!charge.criteres.longueur_atteinte) {
      causes.push("trop court (" + charge.mesures.caracteres + " < " + charge.criteres.longueur_requise + ")");
    }

    if (!charge.criteres.cadence_atteinte) {
      causes.push("trop lent (" + charge.mesures.ecart_moyen_ms + " ms > " + charge.criteres.cadence_requise_ms + " ms)");
    }

    afficherVerdict("Frappe ignoree : " + causes.join(", ") + ". Saisie clavier humaine, pas une douchette.", "neutre");
  }

  function basculerCalibration() {
    const actif = !global.scanner.enCalibration();
    const bouton = element("bouton_calibration");

    global.scanner.calibrer(actif);

    bouton.textContent = actif ? "Arreter la calibration" : "Calibrer la douchette";
    bouton.className = actif ? "bouton bouton--actif" : "bouton";

    afficherVerdict(
      actif
        ? "Calibration active : toute frappe est acceptee, quelle que soit sa cadence. Scanne une etiquette."
        : "Calibration arretee. Les criteres de cadence sont de nouveau appliques.",
      "neutre"
    );
  }

  async function rafraichir() {
    try {
      const reglages = await global.pont.appeler("recuperer_reglages_scanner");
      appliquerReglages(reglages);
    } catch (incident) {
      afficherVerdict("Reglages scanner illisibles : " + incident.message, "alerte");
    }
  }

  function initialiser() {
    element("bouton_calibration").addEventListener("click", basculerCalibration);
    element("scanner_designer").addEventListener("click", designer);
    proposerDouchettes([]);
    global.vueImprimante.initialiser();
    global.vueAfficheur.initialiser();

    global.scanner.surScan(surScan);
    global.scanner.surRejet(surRejet);
    global.scanner.demarrer();
  }

  global.vuePeripheriques = {
    initialiser: initialiser,
    rafraichir: rafraichir,
    proposerDouchettes: proposerDouchettes,
    entrer: function () {
      rafraichir();
      global.vueImprimante.rafraichir();
      global.vueAfficheur.rafraichir();
    }
  };
})(window);
