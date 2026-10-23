(function (global) {
  "use strict";

  const element = global.format.element;

  function rafraichir() {
    element("gerant_nom").value = global.acces.nom();
  }

  async function changerCode() {
    const nouveau = element("gerant_nouveau_code").value;

    if (nouveau !== element("gerant_confirmation").value) {
      global.notifier.alerte("Le nouveau code et sa confirmation ne correspondent pas.");
      return;
    }

    try {
      await global.pont.appeler("modifier_secret_gerant", {
        jeton: global.acces.exiger(),
        ancienCode: element("gerant_ancien_code").value,
        nouveauCode: nouveau
      });

      element("gerant_ancien_code").value = "";
      element("gerant_nouveau_code").value = "";
      element("gerant_confirmation").value = "";
      global.notifier.succes("Code du gerant modifie.");
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function renommer() {
    try {
      const nom = element("gerant_nom").value;

      await global.pont.appeler("renommer_gerant", {
        jeton: global.acces.exiger(),
        nom: nom
      });

      global.acces.definir(global.acces.jeton(), nom.trim());
      element("parametrage_nom_gerant").textContent = nom.trim();
      global.notifier.succes("Gerant renomme.");
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("gerant_changer_code").addEventListener("click", changerCode);
    element("gerant_renommer").addEventListener("click", renommer);
  }

  global.reglageSecurite = {
    initialiser: initialiser,
    rafraichir: rafraichir
  };
})(window);
