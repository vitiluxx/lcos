/*
 * Imprimante du ticket. Le spouleur Windows expose l imprimante integree de
 * la caisse comme n importe quelle autre : la detection consiste a l enumerer
 * puis a retenir celle que le gerant designe.
 *
 * Trois voies : la fenetre d impression de Windows, l envoi direct du texte,
 * et l envoi ESC/POS, seul a commander la coupe du papier, le tiroir-caisse et
 * le gras d une thermique 58 mm.
 */
(function (global) {
  "use strict";

  const element = global.format.element;

  let reglages = null;

  function remplirListe(disponibles, retenue) {
    const liste = element("imprimante_nom");

    global.format.vider(liste);

    if (!disponibles.length) {
      liste.appendChild(new Option("Aucune imprimante detectee", ""));
      liste.disabled = true;
      return;
    }

    liste.disabled = false;

    // Le port est montre avant l etat, parce que l etat ne distingue rien :
    // une imprimante virtuelle se declare `prete` exactement comme la
    // thermique de la caisse. Seul le port dit ou vont les octets.
    disponibles.forEach(function (imprimante) {
      const port = imprimante.port || "port inconnu";
      const suffixe = imprimante.virtuelle
        ? " - virtuelle, " + port + " : aucun ticket n en sortira"
        : " - " + port + (imprimante.hors_ligne ? ", hors ligne" : ", " + imprimante.etat);

      liste.appendChild(new Option(imprimante.nom + suffixe, imprimante.nom));
    });

    liste.value = retenue;

    // Une imprimante materielle est preferee a une virtuelle quand rien n est
    // encore retenu : le premier de la liste du spouleur est souvent `Fax`.
    if (!liste.value) {
      const materielle = disponibles.find(function (imprimante) {
        return !imprimante.virtuelle;
      });

      liste.value = materielle ? materielle.nom : disponibles[0].nom;
    }
  }

  // Les options materielles ne concernent que l ESC/POS : affichees en mode
  // texte, elles feraient croire a une coupe papier qui n arrivera pas.
  function appliquerVisibiliteOptions() {
    element("imprimante_options_escpos").hidden = element("imprimante_mode").value !== "escpos";
  }

  function appliquer(nouveaux) {
    reglages = nouveaux;

    const badge = element("badge_imprimante");
    const nombre = nouveaux.disponibles.length;

    if (nombre) {
      badge.textContent = nombre === 1 ? "detectee" : nombre + " detectees";
      badge.className = "badge badge--succes";
    } else {
      badge.textContent = "aucune";
      badge.className = "badge badge--attente";
    }

    remplirListe(nouveaux.disponibles, nouveaux.imprimante);
    element("imprimante_mode").value = nouveaux.mode;
    element("imprimante_coupe").checked = nouveaux.coupe_papier;
    element("imprimante_tiroir").checked = nouveaux.tiroir_caisse;
    element("imprimante_gras").checked = nouveaux.gras;
    appliquerVisibiliteOptions();
    element("imprimante_etat").textContent = nombre
      ? "detectee le " + (nouveaux.derniere_detection || "--")
      : "branche et allume l imprimante, puis relance la detection";

    global.impression.appliquerVoie(nouveaux.mode);
  }

  async function rafraichir() {
    try {
      appliquer(await global.pont.appeler("recuperer_reglages_impression"));
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function detecter() {
    try {
      appliquer(await global.pont.appeler("detecter_imprimantes"));

      global.notifier.succes(
        reglages.disponibles.length
          ? "Imprimantes relues depuis le spouleur Windows."
          : "Aucune imprimante installee sur ce poste."
      );
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function enregistrer() {
    try {
      appliquer(
        await global.pont.appeler("enregistrer_reglages_impression", {
          jeton: global.acces.exiger(),
          mode: element("imprimante_mode").value,
          imprimante: element("imprimante_nom").value,
          coupePapier: element("imprimante_coupe").checked,
          tiroirCaisse: element("imprimante_tiroir").checked,
          gras: element("imprimante_gras").checked
        })
      );

      global.notifier.succes("Imprimante du ticket enregistree.");
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function tester() {
    try {
      const essai = await global.pont.appeler("tester_impression", {
        jeton: global.acces.exiger()
      });

      element("imprimante_etat").textContent = essai.message;

      // Jamais de vert : le spouleur accepte les octets sans savoir ce qui est
      // au bout du cable. Un port logiciel, lui, est une erreur certaine.
      if (essai.materiel) {
        global.notifier.neutre(essai.message);
      } else {
        global.notifier.alerte(essai.message);
      }
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("imprimante_mode").addEventListener("change", appliquerVisibiliteOptions);
    element("imprimante_detecter").addEventListener("click", detecter);
    element("imprimante_enregistrer").addEventListener("click", enregistrer);
    element("imprimante_tester").addEventListener("click", tester);
  }

  global.vueImprimante = {
    initialiser: initialiser,
    rafraichir: rafraichir
  };
})(window);
