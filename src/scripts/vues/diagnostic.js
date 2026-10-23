(function (global) {
  "use strict";

  function element(identifiant) {
    return document.getElementById(identifiant);
  }

  function afficherErreur(message) {
    const zone = element("zone_erreur");

    zone.textContent = message;
    zone.hidden = false;
  }

  function appliquerEtat(etat) {
    element("etiquette_version").textContent = "version " + etat.version;

    const valeurBase = element("valeur_base");
    valeurBase.textContent = etat.base_operationnelle ? "operationnelle" : "non initialisee";

    element("detail_chemin_base").textContent = etat.chemin_base;
    element("valeur_migrations").textContent = String(etat.migrations_appliquees);
    element("valeur_parametres").textContent = String(etat.parametres_charges);
  }

  async function rafraichir() {
    try {
      const etat = await global.pont.appeler("recuperer_etat_application");
      appliquerEtat(etat);
    } catch (incident) {
      element("valeur_base").textContent = "indisponible";
      afficherErreur("Diagnostic impossible : " + incident.message);
    }
  }

  global.vueDiagnostic = {
    rafraichir: rafraichir
  };
})(window);
