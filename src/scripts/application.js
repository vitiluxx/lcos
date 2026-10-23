(function (global) {
  "use strict";

  function basculerVue(nom) {
    document.querySelectorAll("[data-section]").forEach(function (section) {
      section.hidden = section.dataset.section !== nom;
    });

    document.querySelectorAll(".onglets__bouton").forEach(function (bouton) {
      bouton.classList.toggle("onglets__bouton--actif", bouton.dataset.vue === nom);
    });
  }

  function brancherOnglets() {
    document.querySelectorAll(".onglets__bouton").forEach(function (bouton) {
      bouton.addEventListener("click", function () {
        basculerVue(bouton.dataset.vue);
      });
    });
  }

  function demarrer() {
    brancherOnglets();

    global.vueDiagnostic.rafraichir();
    global.vuePeripheriques.initialiser();
    global.vuePeripheriques.rafraichir();

    // Une lecture reussie amene l'operateur sur la vue qui l'affiche.
    global.scanner.surScan(function () {
      basculerVue("peripheriques");
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", demarrer);
  } else {
    demarrer();
  }
})(window);
