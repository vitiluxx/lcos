/*
 * Droits administrateur du poste. La tache planifiee au niveau le plus eleve
 * evite la demande d autorisation Windows a chaque demarrage : sans elle, le
 * reglage de l horloge echoue ou impose une relance manuelle de LCOS.
 */
(function (global) {
  "use strict";

  const element = global.format.element;

  let etat = null;

  function appliquer(nouveau) {
    etat = nouveau;

    const badge = element("badge_administrateur");

    if (nouveau.eleve) {
      badge.textContent = "administrateur";
      badge.className = "badge badge--succes";
    } else {
      badge.textContent = "droits ordinaires";
      badge.className = "badge badge--attente";
    }

    element("administrateur_detail").textContent = nouveau.tache_installee
      ? "Autorisation posee. Lance LCOS par le raccourci du bureau : " + nouveau.raccourci
      : "Aucune autorisation posee. Programme : " + nouveau.programme;

    element("administrateur_activer").hidden = nouveau.tache_installee;
    element("administrateur_desactiver").hidden = !nouveau.tache_installee;
  }

  async function rafraichir() {
    try {
      appliquer(await global.pont.appeler("recuperer_etat_administrateur"));
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function activer() {
    global.notifier.neutre("Windows va demander l autorisation : accepte la fenetre.");

    try {
      appliquer(
        await global.pont.appeler("activer_lancement_administrateur", {
          jeton: global.acces.exiger()
        })
      );

      global.notifier.succes(
        "Autorisation posee. Ferme LCOS et relance-le par le raccourci du bureau."
      );
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function desactiver() {
    try {
      appliquer(
        await global.pont.appeler("desactiver_lancement_administrateur", {
          jeton: global.acces.exiger()
        })
      );

      global.notifier.succes("Autorisation retiree.");
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("administrateur_activer").addEventListener("click", activer);
    element("administrateur_desactiver").addEventListener("click", desactiver);
  }

  global.reglageAdministrateur = {
    initialiser: initialiser,
    rafraichir: rafraichir,
    eleve: function () {
      return Boolean(etat && etat.eleve);
    }
  };
})(window);
