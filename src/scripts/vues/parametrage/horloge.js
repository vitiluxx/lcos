(function (global) {
  "use strict";

  const element = global.format.element;

  function appliquer(horloge) {
    element("horloge_valeur").textContent = horloge.lisible;
    element("horloge_date").value = horloge.date;
    element("horloge_heure").value = horloge.heure;
  }

  async function rafraichir() {
    try {
      appliquer(await global.pont.appeler("recuperer_horloge"));
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function appliquerReglage() {
    try {
      const horloge = await global.pont.appeler("regler_horloge", {
        jeton: global.acces.exiger(),
        date: element("horloge_date").value,
        heure: element("horloge_heure").value
      });

      appliquer(horloge);
      global.notifier.succes("Horloge de la caisse reglee sur " + horloge.lisible + ".");
      global.application.rafraichirEtat();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function ouvrirPanneauWindows() {
    try {
      await global.pont.appeler("ouvrir_reglages_horloge", {
        jeton: global.acces.exiger()
      });

      global.notifier.neutre("Panneau Windows demande.");
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("horloge_appliquer").addEventListener("click", appliquerReglage);
    element("horloge_panneau").addEventListener("click", ouvrirPanneauWindows);
  }

  global.reglageHorloge = {
    initialiser: initialiser,
    rafraichir: rafraichir
  };
})(window);
