/*
 * Orchestrateur du menu parametrage : verrou par code gerant, puis aiguillage
 * vers les six sections de reglage. Chaque section est un module autonome.
 */
(function (global) {
  "use strict";

  const element = global.format.element;

  const SECTIONS = {
    commerce: function () {
      return global.reglageCommerce;
    },
    horloge: function () {
      return global.reglageHorloge;
    },
    caissiers: function () {
      return global.reglageCaissiers;
    },
    rayons: function () {
      return global.reglageRayons;
    },
    produits: function () {
      return global.reglageProduits;
    },
    securite: function () {
      return global.reglageSecurite;
    }
  };

  let sectionCourante = "commerce";
  let protection = { gerant_configure: false, nom_gerant: "" };

  function basculerSection(nom) {
    if (!SECTIONS[nom]) {
      return;
    }

    sectionCourante = nom;

    document.querySelectorAll("[data-reglage_section]").forEach(function (section) {
      section.hidden = section.dataset.reglage_section !== nom;
    });

    document.querySelectorAll("[data-reglage]").forEach(function (bouton) {
      bouton.classList.toggle(
        "sous_onglets__bouton--actif",
        bouton.dataset.reglage === nom
      );
    });

    const module = SECTIONS[nom]();

    if (module && module.rafraichir) {
      module.rafraichir();
    }

    if (nom === "produits") {
      global.reglageProduits.rafraichirRayons();
    }
  }

  function appliquerVisibilite() {
    const configure = protection.gerant_configure;
    const ouvert = global.acces.actif();

    element("parametrage_initialisation").hidden = configure;
    element("parametrage_verrou").hidden = !configure || ouvert;
    element("parametrage_corps").hidden = !configure || !ouvert;
    element("parametrage_nom_gerant").textContent = global.acces.nom() || protection.nom_gerant;
  }

  async function rafraichir() {
    try {
      protection = await global.pont.appeler("recuperer_etat_protection");
      appliquerVisibilite();

      if (global.acces.actif()) {
        basculerSection(sectionCourante);
      }
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function deverrouiller() {
    try {
      const acces = await global.pont.appeler("ouvrir_session_gerant", {
        code: element("parametrage_code").value
      });

      element("parametrage_code").value = "";
      global.acces.definir(acces.jeton, acces.nom);
      global.notifier.succes("Parametrage deverrouille.");
      await rafraichir();
    } catch (incident) {
      element("parametrage_code").value = "";
      global.notifier.incident(incident);
    }
  }

  async function verrouiller() {
    const jeton = global.acces.jeton();

    global.acces.effacer();

    if (jeton) {
      try {
        await global.pont.appeler("fermer_session_gerant", { jeton: jeton });
      } catch (incident) {
        console.error("revocation du jeton impossible", incident);
      }
    }

    appliquerVisibilite();
    global.notifier.neutre("Parametrage verrouille.");
  }

  async function initialiserGerant() {
    const code = element("initialisation_code").value;

    if (code !== element("initialisation_confirmation").value) {
      global.notifier.alerte("Le code et sa confirmation ne correspondent pas.");
      return;
    }

    try {
      const acces = await global.pont.appeler("initialiser_gerant", {
        nom: element("initialisation_nom").value,
        code: code
      });

      element("initialisation_code").value = "";
      element("initialisation_confirmation").value = "";
      global.acces.definir(acces.jeton, acces.nom);
      global.notifier.succes("Compte gerant cree. La caisse est protegee.");
      await rafraichir();
      global.application.rafraichirEtat();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function surScan(charge) {
    if (!global.acces.actif() || sectionCourante !== "produits") {
      return;
    }

    global.reglageProduits.surScan(charge);
  }

  function initialiser() {
    element("parametrage_deverrouiller").addEventListener("click", deverrouiller);
    element("parametrage_verrouiller").addEventListener("click", verrouiller);
    element("initialisation_valider").addEventListener("click", initialiserGerant);

    element("parametrage_code").addEventListener("keydown", function (evenement) {
      if (evenement.key === "Enter") {
        deverrouiller();
      }
    });

    document.querySelectorAll("[data-reglage]").forEach(function (bouton) {
      bouton.addEventListener("click", function () {
        basculerSection(bouton.dataset.reglage);
      });
    });

    Object.keys(SECTIONS).forEach(function (nom) {
      const module = SECTIONS[nom]();

      if (module && module.initialiser) {
        module.initialiser();
      }
    });
  }

  global.vueParametrage = {
    initialiser: initialiser,
    rafraichir: rafraichir,
    entrer: rafraichir,
    surScan: surScan,
    protegee: function () {
      return protection.gerant_configure;
    }
  };
})(window);
