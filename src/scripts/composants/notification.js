(function (global) {
  "use strict";

  const DUREES = {
    succes: 4000,
    neutre: 5000,
    alerte: 9000
  };

  const MAXIMUM_AFFICHEES = 4;

  // Confirmation de vente quand le recu papier est desactive : un eclair au
  // centre de l ecran, une seconde, que la touche Entree fait partir aussitot.
  const DUREE_ECLAIR_MS = 1000;

  let eclairCourant = null;
  let minuterieEclair = null;

  function zone() {
    return document.getElementById("zone_notifications");
  }

  function afficher(nature, message) {
    const contenant = zone();

    if (!contenant) {
      return;
    }

    while (contenant.childElementCount >= MAXIMUM_AFFICHEES) {
      contenant.removeChild(contenant.firstChild);
    }

    const noeud = document.createElement("p");

    noeud.className = "notification notification--" + nature;
    noeud.textContent = String(message);
    contenant.appendChild(noeud);

    setTimeout(function () {
      if (noeud.parentNode === contenant) {
        contenant.removeChild(noeud);
      }
    }, DUREES[nature] || DUREES.neutre);
  }

  function fermerEclair() {
    if (minuterieEclair !== null) {
      clearTimeout(minuterieEclair);
      minuterieEclair = null;
    }

    if (eclairCourant && eclairCourant.parentNode) {
      eclairCourant.parentNode.removeChild(eclairCourant);
    }

    eclairCourant = null;
  }

  function eclair(message) {
    const contenant = document.getElementById("zone_eclair");

    if (!contenant) {
      return;
    }

    fermerEclair();

    const noeud = document.createElement("p");

    noeud.className = "eclair";
    noeud.textContent = String(message);
    noeud.addEventListener("click", fermerEclair);
    contenant.appendChild(noeud);

    eclairCourant = noeud;
    minuterieEclair = setTimeout(fermerEclair, DUREE_ECLAIR_MS);
  }

  function messageIncident(incident) {
    if (!incident) {
      return "erreur inconnue";
    }

    if (typeof incident === "string") {
      return incident;
    }

    return incident.message || String(incident);
  }

  global.notifier = {
    succes: function (message) {
      afficher("succes", message);
    },
    neutre: function (message) {
      afficher("neutre", message);
    },
    alerte: function (message) {
      afficher("alerte", message);
    },
    incident: function (incident) {
      afficher("alerte", messageIncident(incident));
    },
    eclair: eclair,
    fermerEclair: fermerEclair,
    eclairAffiche: function () {
      return Boolean(eclairCourant);
    }
  };
})(window);
