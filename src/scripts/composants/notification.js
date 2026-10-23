(function (global) {
  "use strict";

  const DUREES = {
    succes: 4000,
    neutre: 5000,
    alerte: 9000
  };

  const MAXIMUM_AFFICHEES = 4;

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
    }
  };
})(window);
