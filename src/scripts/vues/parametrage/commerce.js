(function (global) {
  "use strict";

  const element = global.format.element;

  function appliquer(reglages) {
    element("commerce_nom").value = reglages.nom_commerce;
    element("commerce_devise").value = reglages.devise;
    element("ticket_actif").checked = reglages.ticket_actif;
    element("ticket_impression_auto").checked = reglages.ticket_impression_auto;
    element("ticket_colonnes").value = String(reglages.ticket_largeur_colonnes);
    element("ticket_entete").value = reglages.ticket_entete;
    element("ticket_pied").value = reglages.ticket_pied;
  }

  async function rafraichir() {
    try {
      appliquer(await global.pont.appeler("recuperer_reglages"));
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function enregistrer() {
    try {
      const reglages = await global.pont.appeler("enregistrer_reglages", {
        jeton: global.acces.exiger(),
        nomCommerce: element("commerce_nom").value,
        devise: element("commerce_devise").value,
        ticketActif: element("ticket_actif").checked,
        ticketImpressionAuto: element("ticket_impression_auto").checked,
        ticketLargeurColonnes: global.format.entier(element("ticket_colonnes").value, 32),
        ticketEntete: element("ticket_entete").value,
        ticketPied: element("ticket_pied").value
      });

      appliquer(reglages);
      global.notifier.succes("Reglages enregistres.");
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("commerce_enregistrer").addEventListener("click", enregistrer);
  }

  global.reglageCommerce = {
    initialiser: initialiser,
    rafraichir: rafraichir
  };
})(window);
