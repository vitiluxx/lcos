/*
 * Droit d usage : essai gratuit puis licence signee.
 *
 * Deux portes sur le meme etat :
 *   - la vue Licence, seul ecran accessible quand la caisse est fermee ;
 *   - la section Licence du parametrage, pour consulter et remplacer en temps
 *     normal.
 * La verification est faite par le noyau : ici on affiche, on saisit, on
 * transmet.
 */
(function (global) {
  "use strict";

  const element = global.format.element;

  let etat = null;

  function libelleValidite(courant) {
    if (!courant.sous_licence) {
      return courant.autorise
        ? courant.jours_restants + " jour(s) d essai"
        : "essai termine";
    }

    if (courant.perpetuelle) {
      return "perpetuelle";
    }

    if (!courant.expire_le) {
      return "--";
    }

    return "jusqu au " + courant.expire_le;
  }

  function natureBadge(courant) {
    if (!courant.autorise) {
      return "badge badge--alerte";
    }

    return courant.sous_licence ? "badge badge--succes" : "badge badge--attente";
  }

  function libelleBadge(courant) {
    if (courant.statut === "sous_licence") {
      return courant.perpetuelle ? "sous licence" : courant.jours_restants + " j";
    }

    if (courant.statut === "essai") {
      return "essai : " + courant.jours_restants + " j";
    }

    if (courant.statut === "essai_termine") {
      return "essai termine";
    }

    if (courant.statut === "licence_expiree") {
      return "licence expiree";
    }

    if (courant.statut === "licence_autre_poste") {
      return "autre poste";
    }

    return "licence refusee";
  }

  function poser(suffixe, courant) {
    element(suffixe + "badge").textContent = libelleBadge(courant);
    element(suffixe + "badge").className = natureBadge(courant);
    element(suffixe + "message").textContent = courant.message;
    element(suffixe + "titulaire").textContent = courant.titulaire || "aucun";
    element(suffixe + "numero").textContent = courant.numero || "--";
    element(suffixe + "validite").textContent = libelleValidite(courant);
    element(suffixe + "poste").textContent = courant.poste;
  }

  // L entete porte un rappel discret pendant l essai, et une alerte quand la
  // caisse est fermee. Sous licence perpetuelle, rien : le commerce n a pas a
  // etre rappele a l ordre tous les matins.
  function poserEntete(courant) {
    const badge = element("etiquette_licence");
    const discret = courant.statut === "sous_licence" && courant.perpetuelle;

    badge.hidden = discret;
    badge.textContent = libelleBadge(courant);
    badge.className = natureBadge(courant) + " entete__licence";
  }

  function appliquer(courant) {
    etat = courant;

    poser("licence_", courant);
    poser("reglage_licence_", courant);
    poserEntete(courant);

    element("licence_titre").textContent = courant.autorise
      ? "Licence de LCOS"
      : "Caisse fermee : licence requise";
    element("licence_retirer").hidden = !courant.sous_licence || !global.acces.actif();
  }

  async function rafraichir() {
    try {
      appliquer(await global.pont.appeler("recuperer_etat_licence"));
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function activer() {
    const cle = element("licence_cle").value;

    if (!cle.trim()) {
      global.notifier.alerte("Colle d abord la cle de licence.");
      return;
    }

    try {
      appliquer(await global.pont.appeler("activer_licence", { cle: cle }));

      element("licence_cle").value = "";
      global.notifier.succes(
        "Licence acceptee : " + (etat.titulaire || "poste active") + "."
      );

      // La caisse rouvre : tout l etat de l application est relu.
      await global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function retirer() {
    try {
      appliquer(
        await global.pont.appeler("retirer_licence", { jeton: global.acces.exiger() })
      );

      global.notifier.neutre("Licence retiree de ce poste.");
      await global.application.rafraichirEtat();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("licence_activer").addEventListener("click", activer);
    element("licence_retirer").addEventListener("click", retirer);
    element("etiquette_licence").addEventListener("click", function () {
      global.application.allerA("licence");
    });
    element("reglage_licence_aller").addEventListener("click", function () {
      global.application.allerA("licence");
    });

    element("licence_cle").addEventListener("keydown", function (evenement) {
      if (evenement.key === "Enter" && !evenement.shiftKey) {
        evenement.preventDefault();
        activer();
      }
    });
  }

  function entrer() {
    rafraichir();
    element("licence_cle").focus();
  }

  global.vueLicence = {
    initialiser: initialiser,
    rafraichir: rafraichir,
    appliquer: appliquer,
    entrer: entrer,
    autorise: function () {
      return !etat || etat.autorise;
    },

    // Etat inconnu : on ne bride rien a l ecran sur une supposition. Les
    // plafonds de l essai sont tenus par le noyau, qui refuse pour de vrai.
    //
    // Une licence posee mais refusee (illisible, expiree, autre poste) ne vaut
    // pas la version complete : c est le meme calcul que dans le noyau.
    sousLicence: function () {
      return !etat || Boolean(etat.sous_licence && etat.autorise);
    },

    quotaAtteint: function (presents, maximum) {
      return (
        Boolean(etat) &&
        !(etat.sous_licence && etat.autorise) &&
        presents >= maximum
      );
    }
  };

  // Section du parametrage : meme etat, pas de seconde initialisation.
  global.reglageLicence = {
    rafraichir: rafraichir
  };
})(window);
