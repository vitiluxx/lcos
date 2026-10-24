/*
 * Aiguilleur de l application : bascule des trois vues, etat d entete, routage
 * des scans vers la vue concernee. Les rapports, les peripheriques et le
 * diagnostic ne sont plus des vues : ils vivent dans le parametrage, derriere
 * le code du gerant.
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
    licence: function () {
      return global.vueLicence;
    }
  };

  const LIBELLES = {
    caisse: "Caisse",
    session: "Session",
    parametrage: "Parametrage",
    licence: "Licence"
  };

  const element = global.format.element;

  let vueCourante = "caisse";

  // Droit d usage : essai en cours ou licence valide. Faux, la caisse est
  // fermee et la vue Licence devient le seul ecran accessible.
  let usageAutorise = true;

  function menuOuvert() {
    return !element("menu_liste").hidden;
  }

  function ouvrirMenu(ouvert) {
    element("menu_liste").hidden = !ouvert;
    element("menu_bascule").setAttribute("aria-expanded", ouvert ? "true" : "false");
  }

  function basculerVue(nom) {
    if (!VUES[nom]) {
      return;
    }

    if (!usageAutorise && nom !== "licence") {
      ouvrirMenu(false);
      global.notifier.alerte(
        "Caisse fermee : insere une licence pour rouvrir les autres ecrans."
      );

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

    document.querySelectorAll(".menu_vues__element").forEach(function (bouton) {
      bouton.classList.toggle("menu_vues__element--actif", bouton.dataset.vue === nom);
    });

    element("menu_vue_courante").textContent = LIBELLES[nom];
    ouvrirMenu(false);

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
    appliquerLicence(etat.licence);
  }

  // Seule la vue Licence reste au menu quand le droit d usage manque : les
  // donnees ne sont pas touchees, elles attendent la licence.
  function appliquerLicence(etatLicence) {
    const etaitAutorise = usageAutorise;

    usageAutorise = etatLicence.autorise;
    global.vueLicence.appliquer(etatLicence);

    document.querySelectorAll(".menu_vues__element").forEach(function (bouton) {
      const estLicence = bouton.dataset.vue === "licence";

      bouton.hidden = usageAutorise ? estLicence : !estLicence;
    });

    if (!usageAutorise && vueCourante !== "licence") {
      basculerVue("licence");
      return;
    }

    if (usageAutorise && !etaitAutorise && vueCourante === "licence") {
      basculerVue("caisse");
    }
  }

  async function rafraichirEtat() {
    try {
      appliquerEtat(await global.pont.appeler("recuperer_etat_application"));
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function routerScan(charge) {
    if (vueCourante === "parametrage") {
      global.vueParametrage.surScan(charge);
      return;
    }

    if (vueCourante !== "caisse") {
      basculerVue("caisse");
    }

    global.vueCaisse.surScan(charge);
  }

  function brancherMenu() {
    element("menu_bascule").addEventListener("click", function (evenement) {
      evenement.stopPropagation();
      ouvrirMenu(!menuOuvert());
    });

    document.querySelectorAll(".menu_vues__element").forEach(function (bouton) {
      bouton.addEventListener("click", function () {
        basculerVue(bouton.dataset.vue);
      });
    });

    // Un clic ailleurs ou la touche Echap referment le menu : sur un ecran
    // tactile, le caissier n a pas de seconde chance pour viser l icone.
    document.addEventListener("click", function (evenement) {
      if (!menuOuvert() || element("menu_vues_racine").contains(evenement.target)) {
        return;
      }

      ouvrirMenu(false);
    });

    document.addEventListener("keydown", function (evenement) {
      if (evenement.key === "Escape" && menuOuvert()) {
        ouvrirMenu(false);
      }
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

    brancherMenu();

    global.vueLicence.initialiser();
    global.vueCaisse.initialiser();
    global.vueSession.initialiser();
    global.vueParametrage.initialiser();

    global.scanner.surScan(routerScan);

    await rafraichirEtat();
    await global.vueParametrage.rafraichir();

    // Reglages lisibles sans le code du gerant : la cadence de la douchette et
    // la voie d impression doivent etre connues des le premier scan, meme si
    // personne n ouvre le parametrage.
    global.vuePeripheriques.rafraichir();
    global.vueImprimante.rafraichir();

    // Caisse fermee : licence d abord. Sinon, caisse non protegee : creation du
    // compte gerant. Sinon : ecran de travail.
    if (!usageAutorise) {
      basculerVue("licence");
    } else {
      basculerVue(global.vueParametrage.protegee() ? "caisse" : "parametrage");
    }

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
