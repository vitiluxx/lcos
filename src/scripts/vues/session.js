(function (global) {
  "use strict";

  let caissiers = [];
  let caissierChoisi = null;
  let session = null;

  const element = global.format.element;

  function afficherEtat() {
    element("session_etat").textContent = session ? "ouverte" : "fermee";
    element("session_caissier").textContent = session ? session.caissier_nom : "--";
    element("session_ouverte_le").textContent = session ? session.ouverte_le : "--";
    element("session_ventes").textContent = session ? String(session.nombre_ventes) : "0";
    element("session_total").textContent = global.format.monnaie(
      session ? session.total_centimes : 0
    );
    element("session_fond").textContent = global.format.monnaie(
      session ? session.fond_centimes : 0
    );

    element("session_ouverture").hidden = Boolean(session);
    element("session_fermeture").hidden = !session;
  }

  function afficherCaissiers() {
    const zone = element("session_liste_caissiers");

    global.format.vider(zone);

    if (!caissiers.length) {
      zone.appendChild(
        global.format.creer(
          "p",
          "caisse__aide",
          "Aucun caissier actif : cree-en un dans Parametrage > Caissiers."
        )
      );
      return;
    }

    caissiers.forEach(function (caissier) {
      const pastille = global.format.creer("button", "pastille_caissier", caissier.nom);

      pastille.type = "button";

      if (caissierChoisi === caissier.identifiant) {
        pastille.className = "pastille_caissier pastille_caissier--actif";
      }

      pastille.addEventListener("click", function () {
        caissierChoisi = caissier.identifiant;
        afficherCaissiers();
        element("session_code").focus();
      });

      zone.appendChild(pastille);
    });
  }

  async function ouvrir() {
    if (!caissierChoisi) {
      global.notifier.alerte("Choisis d abord un caissier.");
      return;
    }

    try {
      session = await global.pont.appeler("ouvrir_session_caissier", {
        caissier: caissierChoisi,
        code: element("session_code").value,
        fond: element("session_fond_saisie").value
      });

      element("session_code").value = "";
      afficherEtat();
      global.notifier.succes("Session ouverte pour " + session.caissier_nom + ".");
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function fermer() {
    try {
      const fermee = await global.pont.appeler("fermer_session_caissier", {
        code: element("session_code_fermeture").value
      });

      element("session_code_fermeture").value = "";
      session = null;
      afficherEtat();
      global.notifier.succes(
        "Session de " +
          fermee.caissier_nom +
          " fermee. Encaisse : " +
          global.format.monnaie(fermee.total_centimes)
      );
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function rafraichir() {
    try {
      const [etat, actifs] = await Promise.all([
        global.pont.appeler("recuperer_session_courante"),
        global.pont.appeler("lister_caissiers_actifs")
      ]);

      session = etat.ouverte ? etat.session : null;
      caissiers = actifs;

      if (!caissiers.some(function (caissier) {
          return caissier.identifiant === caissierChoisi;
        })) {
        caissierChoisi = caissiers.length === 1 ? caissiers[0].identifiant : null;
      }

      afficherEtat();
      afficherCaissiers();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("session_ouvrir").addEventListener("click", ouvrir);
    element("session_fermer").addEventListener("click", fermer);

    element("session_code").addEventListener("keydown", function (evenement) {
      if (evenement.key === "Enter") {
        ouvrir();
      }
    });

    element("session_code_fermeture").addEventListener("keydown", function (evenement) {
      if (evenement.key === "Enter") {
        fermer();
      }
    });
  }

  global.vueSession = {
    initialiser: initialiser,
    rafraichir: rafraichir,
    entrer: rafraichir
  };
})(window);
