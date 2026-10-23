(function (global) {
  "use strict";

  // Le texte part dans une zone unique que la feuille d impression est seule
  // a rendre visible : la sortie papier est exactement le texte verifie cote
  // Rust, sans mise en page du navigateur.
  function imprimer(texte) {
    const zone = document.getElementById("zone_impression");

    if (!zone) {
      return;
    }

    zone.textContent = String(texte || "");

    global.focus();
    global.print();
  }

  global.impression = {
    imprimer: imprimer
  };
})(window);
