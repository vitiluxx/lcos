(function (global) {
  "use strict";

  const element = global.format.element;

  function afficherErreur(message) {
    const zone = element("zone_erreur");

    zone.textContent = message;
    zone.hidden = false;
  }

  function appliquer(etat) {
    element("valeur_base").textContent = etat.base_operationnelle
      ? "operationnelle"
      : "non initialisee";
    element("detail_chemin_base").textContent = etat.chemin_base;
    element("valeur_migrations").textContent = String(etat.migrations_appliquees);
    element("valeur_parametres").textContent = String(etat.parametres_charges);
    element("valeur_caissiers").textContent = String(etat.nombre_caissiers);
    element("valeur_catalogue").textContent =
      etat.nombre_rayons + " rayons / " + etat.nombre_produits + " produits";
  }

  async function rafraichir() {
    try {
      appliquer(await global.pont.appeler("recuperer_etat_application"));
    } catch (incident) {
      element("valeur_base").textContent = "indisponible";
      afficherErreur("Diagnostic impossible : " + incident.message);
    }
  }

  global.vueDiagnostic = {
    appliquer: appliquer,
    rafraichir: rafraichir,
    entrer: rafraichir
  };
})(window);
