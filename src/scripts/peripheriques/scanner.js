/*
 * Douchette code-barres USB en mode clavier (keyboard wedge).
 * Le lecteur se declare comme un clavier HID : il tape les caracteres du code
 * puis un suffixe (Entree ou Tabulation). Aucun pilote n'est requis.
 * La distinction avec une frappe humaine repose sur la cadence entre touches.
 */
(function (global) {
  "use strict";

  const SUFFIXES = ["Enter", "Tab"];

  const reglages = {
    actif: true,
    delai_max_entre_touches: 50,
    longueur_minimale: 4,
    delai_inactivite: 120
  };

  let tampon = [];
  let instants = [];
  let minuterie = null;
  let calibration = false;

  const abonnes = {
    scan: [],
    rejet: []
  };

  function maintenant() {
    return global.performance && global.performance.now
      ? global.performance.now()
      : Date.now();
  }

  function reinitialiser() {
    tampon = [];
    instants = [];

    if (minuterie !== null) {
      clearTimeout(minuterie);
      minuterie = null;
    }
  }

  function mesurer() {
    const ecarts = [];

    for (let rang = 1; rang < instants.length; rang += 1) {
      ecarts.push(instants[rang] - instants[rang - 1]);
    }

    const total = ecarts.reduce(function (somme, ecart) {
      return somme + ecart;
    }, 0);

    return {
      caracteres: tampon.length,
      ecart_moyen_ms: ecarts.length ? Math.round((total / ecarts.length) * 10) / 10 : 0,
      ecart_max_ms: ecarts.length ? Math.round(Math.max.apply(null, ecarts) * 10) / 10 : 0,
      duree_ms: instants.length > 1
        ? Math.round((instants[instants.length - 1] - instants[0]) * 10) / 10
        : 0
    };
  }

  function diffuser(canal, charge) {
    abonnes[canal].forEach(function (rappel) {
      try {
        rappel(charge);
      } catch (incident) {
        console.error("abonne scanner en echec", incident);
      }
    });
  }

  function cloturer(suffixe, evenement) {
    if (!tampon.length) {
      reinitialiser();
      return;
    }

    const code = tampon.join("");
    const mesures = mesurer();
    const assezLong = mesures.caracteres >= reglages.longueur_minimale;
    const assezRapide = mesures.ecart_moyen_ms <= reglages.delai_max_entre_touches;
    const accepte = calibration || (assezLong && assezRapide);

    reinitialiser();

    const charge = {
      code: code,
      suffixe: suffixe,
      calibration: calibration,
      mesures: mesures,
      criteres: {
        longueur_requise: reglages.longueur_minimale,
        cadence_requise_ms: reglages.delai_max_entre_touches,
        longueur_atteinte: assezLong,
        cadence_atteinte: assezRapide
      }
    };

    if (!accepte) {
      diffuser("rejet", charge);
      return;
    }

    if (evenement) {
      evenement.preventDefault();
    }

    diffuser("scan", charge);
  }

  function armerInactivite() {
    if (minuterie !== null) {
      clearTimeout(minuterie);
    }

    // Certaines douchettes n'emettent aucun suffixe : on cloture sur silence.
    minuterie = setTimeout(function () {
      cloturer("silence", null);
    }, reglages.delai_inactivite);
  }

  // Une saisie au clavier dans un formulaire n est pas un scan. Exception :
  // les champs marques data-douchette, ou l on veut justement lire une
  // etiquette (saisie du code-barres d un produit).
  function cibleIgnoree(evenement) {
    const cible = evenement.target;

    if (!cible || typeof cible.closest !== "function") {
      return false;
    }

    if (cible.dataset && cible.dataset.douchette === "oui") {
      return false;
    }

    return Boolean(cible.closest('[data-saisie="clavier"]'));
  }

  function surTouche(evenement) {
    if (!reglages.actif) {
      return;
    }

    if (cibleIgnoree(evenement)) {
      reinitialiser();
      return;
    }

    if (evenement.ctrlKey || evenement.altKey || evenement.metaKey) {
      return;
    }

    if (SUFFIXES.indexOf(evenement.key) !== -1) {
      cloturer(evenement.key, evenement);
      return;
    }

    if (evenement.key.length !== 1) {
      return;
    }

    const instant = maintenant();

    if (instants.length && instant - instants[instants.length - 1] > reglages.delai_inactivite) {
      reinitialiser();
    }

    tampon.push(evenement.key);
    instants.push(instant);
    armerInactivite();
  }

  function appliquerReglages(nouveaux) {
    if (!nouveaux) {
      return;
    }

    Object.keys(reglages).forEach(function (cle) {
      if (Object.prototype.hasOwnProperty.call(nouveaux, cle)) {
        reglages[cle] = nouveaux[cle];
      }
    });
  }

  function signature(charge) {
    return [
      "wedge",
      charge.suffixe,
      charge.mesures.caracteres + "c",
      charge.mesures.ecart_moyen_ms + "ms"
    ].join("/");
  }

  global.scanner = {
    demarrer: function () {
      document.addEventListener("keydown", surTouche, true);
    },
    appliquerReglages: appliquerReglages,
    reglages: function () {
      return Object.assign({}, reglages);
    },
    oublier: reinitialiser,
    calibrer: function (actif) {
      calibration = Boolean(actif);
      reinitialiser();
    },
    enCalibration: function () {
      return calibration;
    },
    signature: signature,
    surScan: function (rappel) {
      abonnes.scan.push(rappel);
    },
    surRejet: function (rappel) {
      abonnes.rejet.push(rappel);
    }
  };
})(window);
