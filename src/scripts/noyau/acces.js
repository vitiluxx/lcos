(function (global) {
  "use strict";

  // Le jeton du gerant ne vit qu en memoire : fermer LCOS le perd, et aucun
  // stockage navigateur ne peut le reveler.
  let jeton = null;
  let nom = "";

  const abonnes = [];

  function diffuser() {
    abonnes.forEach(function (rappel) {
      try {
        rappel(Boolean(jeton));
      } catch (incident) {
        console.error("abonne acces en echec", incident);
      }
    });
  }

  global.acces = {
    definir: function (nouveauJeton, nomGerant) {
      jeton = nouveauJeton;
      nom = nomGerant || "";
      diffuser();
    },
    effacer: function () {
      jeton = null;
      nom = "";
      diffuser();
    },
    jeton: function () {
      return jeton;
    },
    nom: function () {
      return nom;
    },
    actif: function () {
      return Boolean(jeton);
    },
    exiger: function () {
      if (!jeton) {
        throw new Error("menu protege : saisir le code du gerant");
      }

      return jeton;
    },
    surChangement: function (rappel) {
      abonnes.push(rappel);
    }
  };
})(window);
