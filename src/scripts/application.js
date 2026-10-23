/*
 * Aiguilleur de l application : bascule des vues, etat d entete, routage des
 * scans vers la vue concernee.
 */
(function (global) {
  "use strict";

  const RAFRAICHISSEMENT_ETAT_MS = 20000;

  const VUES = {
    caisse: function () {
      return global.vueCaisse;
    },
    session: function () {
      return global.vueSession;
    },
    parametrage: function () {
      return global.vueParametrage;
    },
    rapports: function () {
      return global.vueRapports;
    },
    peripheriques: function () {
      return global.vuePeripheriques;
    },
    diagnostic: function () {
      return global.vueDiagnostic;
    }
  };

  const element = global.format.element;

  let vueCourante = "caisse";

  function basculerVue(nom) {
    if (!VUES[nom]) {
      return;
    }

    const precedente = VUES[vueCourante]();

    if (precedente && precedente.sortir) {
      precedente.sortir();
    }

    vueCourante = nom;

    document.querySelectorAll("[data-section]").forEach(function (section) {
      section.hidden = section.dataset.section !== nom;
    });

    document.querySelectorAll(".onglets__bouton").forEach(function (bouton) {
      bouton.classList.toggle("onglets__bouton--actif", bouton.dataset.vue === nom);
    });

    const module = VUES[nom]();

    if (module && module.entrer) {
      module.entrer();
    }
  }

  function appliquerEtat(etat) {
    element("etiquette_version").textContent = "version " + etat.version;
    element("etiquette_horloge").textContent = etat.horodatage;
    element("entete_commerce").textContent = etat.nom_commerce;

    const session = element("etiquette_session");

    if (etat.session_ouverte) {
      session.textContent = "caisse ouverte : " + etat.caissier_en_poste;
      session.className = "badge badge--succes";
    } else {
      session.textContent = "session fermee";
      session.className = "badge badge--attente";
    }

    global.vueDiagnostic.appliquer(etat);
  }

  async function rafraichirEtat() {
    try {
      appliquerEtat(await global.pont.appeler("recuperer_etat_application"));
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function routerScan(charge) {
    if (vueCourante === "peripheriques") {
      return;
    }

    if (vueCourante === "parametrage") {
      global.vueParametrage.surScan(charge);
      return;
    }

    if (vueCourante !== "caisse") {
      basculerVue("caisse");
    }

    global.vueCaisse.surScan(charge);
  }

  function brancherOnglets() {
    document.querySelectorAll(".onglets__bouton").forEach(function (bouton) {
      bouton.addEventListener("click", function () {
        basculerVue(bouton.dataset.vue);
      });
    });

    element("caisse_aller_session").addEventListener("click", function () {
      basculerVue("session");
    });
  }

  async function demarrer() {
    if (!global.pont.disponible()) {
      global.notifier.alerte(
        "Pont natif indisponible : lancer LCOS installe, pas la page seule."
      );
    }

    brancherOnglets();

    global.vueCaisse.initialiser();
    global.vueSession.initialiser();
    global.vueParametrage.initialiser();
    global.vueRapports.initialiser();
    global.vuePeripheriques.initialiser();

    global.scanner.surScan(routerScan);

    await rafraichirEtat();
    await global.vueParametrage.rafraichir();
    global.vuePeripheriques.rafraichir();

    // Caisse non protegee : le premier ecran est la creation du compte gerant.
    basculerVue(global.vueParametrage.protegee() ? "caisse" : "parametrage");

    setInterval(rafraichirEtat, RAFRAICHISSEMENT_ETAT_MS);
  }

  global.application = {
    allerA: basculerVue,
    rafraichirEtat: rafraichirEtat,
    vueCourante: function () {
      return vueCourante;
    }
  };

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", demarrer);
  } else {
    demarrer();
  }
})(window);
