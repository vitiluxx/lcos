/*
 * Identite du commerce, devise et impression des recus. La devise porte une
 * consequence lourde : sans subdivision, toute la caisse passe en nombres
 * entiers, donc le reglage des decimales vit a cote d elle, pas noye dans le
 * reste.
 */
(function (global) {
  "use strict";

  const element = global.format.element;

  // Memes devises que le noyau Rust : la proposition faite a l ecran doit etre
  // celle que le noyau aurait choisie par defaut.
  const DEVISES_SANS_DECIMALE = ["FCFA", "CFA", "F CFA", "XOF", "XAF", "KMF", "GNF"];

  function sansDecimale(devise) {
    return DEVISES_SANS_DECIMALE.indexOf(String(devise || "").trim().toUpperCase()) !== -1;
  }

  function afficherBadgeRecu(actif) {
    const badge = element("badge_impression_recu");

    badge.textContent = actif ? "activee" : "desactivee";
    badge.className = actif ? "badge badge--succes" : "badge badge--attente";
  }

  function afficherBadgeAnnulation(actif) {
    const badge = element("badge_annulation_vente");

    badge.textContent = actif ? "activee" : "desactivee";
    badge.className = actif ? "badge badge--alerte" : "badge badge--attente";
  }

  function appliquer(reglages) {
    element("commerce_nom").value = reglages.nom_commerce;
    element("commerce_devise").value = reglages.devise;
    element("commerce_decimales").value = reglages.decimales > 0 ? "2" : "0";
    element("ticket_actif").checked = reglages.ticket_actif;
    element("impression_recu").checked = reglages.impression_recu;
    element("annulation_vente").checked = reglages.annulation_vente;
    element("ticket_colonnes").value = String(reglages.ticket_largeur_colonnes);
    element("ticket_entete").value = reglages.ticket_entete;
    element("ticket_pied").value = reglages.ticket_pied;

    afficherBadgeRecu(reglages.impression_recu);
    afficherBadgeAnnulation(reglages.annulation_vente);
    global.format.definirDecimales(reglages.decimales);
  }

  // Une devise sans subdivision propose d elle meme les entiers pleins ; le
  // gerant garde le dernier mot sur la liste.
  function surDevise() {
    const devise = element("commerce_devise").value;
    const aide = element("commerce_decimales_aide");

    if (sansDecimale(devise)) {
      element("commerce_decimales").value = "0";
      aide.textContent = devise.trim().toUpperCase() + " : entiers pleins proposes.";
      return;
    }

    element("commerce_decimales").value = "2";
    aide.textContent = "Devise subdivisee : deux decimales proposees.";
  }

  function rafraichirAffichages() {
    global.application.rafraichirEtat();
    global.vueCaisse.rafraichir();
    global.vueRapports.rafraichir();
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
        decimales: global.format.entier(element("commerce_decimales").value, 2),
        ticketActif: element("ticket_actif").checked,
        impressionRecu: element("impression_recu").checked,
        ticketLargeurColonnes: global.format.entier(element("ticket_colonnes").value, 32),
        ticketEntete: element("ticket_entete").value,
        ticketPied: element("ticket_pied").value
      });

      appliquer(reglages);
      global.notifier.succes("Reglages enregistres.");
      rafraichirAffichages();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  // Bascule seule de l impression des recus : rien d autre n est enregistre,
  // c est un geste quotidien.
  async function appliquerImpressionRecu() {
    const actif = element("impression_recu").checked;

    try {
      const reglages = await global.pont.appeler("basculer_impression_recu", {
        jeton: global.acces.exiger(),
        actif: actif
      });

      appliquer(reglages);
      global.notifier.succes(
        actif
          ? "Impression du recu activee apres chaque vente."
          : "Impression du recu desactivee. Le rapport Z reste imprime."
      );
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  // Ouverture de l annulation des ventes. Geste rare et lourd de consequence :
  // il reste isole de l enregistrement des autres reglages.
  async function appliquerAnnulationVente() {
    const actif = element("annulation_vente").checked;

    try {
      const reglages = await global.pont.appeler("basculer_annulation_vente", {
        jeton: global.acces.exiger(),
        actif: actif
      });

      appliquer(reglages);
      global.notifier.succes(
        actif
          ? "Annulation des ventes autorisee depuis l ecran Session."
          : "Annulation des ventes interdite. Les ventes deja faites sont definitives."
      );
      global.vueSession.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("commerce_enregistrer").addEventListener("click", enregistrer);
    element("commerce_devise").addEventListener("change", surDevise);
    element("impression_recu_appliquer").addEventListener("click", appliquerImpressionRecu);
    element("annulation_vente_appliquer").addEventListener("click", appliquerAnnulationVente);
  }

  global.reglageCommerce = {
    initialiser: initialiser,
    rafraichir: rafraichir
  };
})(window);
