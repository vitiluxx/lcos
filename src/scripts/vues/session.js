/*
 * Ecran de session. Il porte aussi le journal des ventes : c est le seul
 * endroit d ou une vente deja encaissee peut etre annulee, et seulement si le
 * gerant a ouvert cette porte dans le Parametrage.
 */
(function (global) {
  "use strict";

  let caissiers = [];
  let caissierChoisi = null;
  let session = null;
  let ventes = [];
  let annulationActive = false;
  let venteChoisie = null;
  let active = false;

  const element = global.format.element;

  function afficherEtat() {
    element("session_etat").textContent = session ? "ouverte" : "fermee";
    element("session_caissier").textContent = session ? session.caissier_nom : "--";
    element("session_ouverte_le").textContent = session ? session.ouverte_le : "--";
    element("session_ventes").textContent = session ? String(session.nombre_ventes) : "0";
    element("session_total").textContent = global.format.monnaie(
      session ? session.total_centimes : 0
    );
    element("session_fond").textContent = global.format.monnaie(
      session ? session.fond_centimes : 0
    );

    element("session_ouverture").hidden = Boolean(session);
    element("session_fermeture").hidden = !session;
  }

  function afficherCaissiers() {
    const zone = element("session_liste_caissiers");

    global.format.vider(zone);

    if (!caissiers.length) {
      zone.appendChild(
        global.format.creer(
          "p",
          "caisse__aide",
          "Aucun caissier actif : cree-en un dans Parametrage > Caissiers."
        )
      );
      return;
    }

    caissiers.forEach(function (caissier) {
      const pastille = global.format.creer("button", "pastille_caissier", caissier.nom);

      pastille.type = "button";

      if (caissierChoisi === caissier.identifiant) {
        pastille.className = "pastille_caissier pastille_caissier--actif";
      }

      pastille.addEventListener("click", function () {
        caissierChoisi = caissier.identifiant;
        afficherCaissiers();
        element("session_code").focus();
      });

      zone.appendChild(pastille);
    });
  }

  function venteParIdentifiant(identifiant) {
    return ventes.find(function (vente) {
      return vente.identifiant === identifiant;
    }) || null;
  }

  function afficherBadgeAnnulation() {
    const badge = element("session_badge_annulation");

    badge.textContent = annulationActive
      ? "annulation autorisee"
      : "annulation desactivee";
    badge.className = annulationActive ? "badge badge--alerte" : "badge badge--attente";

    element("session_journal_intro").textContent = annulationActive
      ? "Les quatre ventes les plus recentes sont visibles : descends pour voir les " +
        "precedentes. Croix ou touche Suppr sur une vente choisie pour l annuler."
      : "Les quatre ventes les plus recentes sont visibles : descends dans la liste " +
        "pour voir les precedentes.";
  }

  function marquerChoix() {
    document.querySelectorAll("#session_journal tr[data-vente]").forEach(function (ligne) {
      const vise = Number(ligne.dataset.vente) === venteChoisie;

      ligne.dataset.selectionne = vise ? "oui" : "non";
    });
  }

  function choisir(identifiant) {
    venteChoisie = identifiant;
    marquerChoix();
  }

  function afficherJournal() {
    const corps = element("session_journal");

    global.format.vider(corps);
    afficherBadgeAnnulation();

    if (!ventes.length) {
      corps.appendChild(global.format.ligneVide(6, "Aucune vente enregistree."));
      cacherAnnulation();
      return;
    }

    ventes.forEach(function (vente) {
      const ligne = global.format.creer("tr");

      ligne.dataset.vente = String(vente.identifiant);

      ligne.appendChild(global.format.cellule("no " + vente.numero_ticket));
      ligne.appendChild(global.format.cellule(vente.horodatage));
      ligne.appendChild(global.format.cellule(vente.caissier));
      ligne.appendChild(
        global.format.cellule(String(vente.nombre_articles), "tableau__chiffre")
      );
      ligne.appendChild(
        global.format.cellule(global.format.monnaie(vente.total_centimes), "tableau__chiffre")
      );

      const actions = global.format.creer("td");

      if (annulationActive) {
        const groupe = global.format.creer("div", "tableau__actions");
        const croix = global.format.creer(
          "button",
          "bouton bouton--petit bouton--danger",
          "x"
        );

        croix.type = "button";
        croix.title = "Annuler la vente no " + vente.numero_ticket;
        croix.addEventListener("click", function (evenement) {
          evenement.stopPropagation();
          choisir(vente.identifiant);
          demanderAnnulation(vente.identifiant);
        });

        groupe.appendChild(croix);
        actions.appendChild(groupe);
      }

      ligne.appendChild(actions);
      ligne.addEventListener("click", function () {
        choisir(vente.identifiant);
      });

      corps.appendChild(ligne);
    });

    if (venteChoisie !== null && !venteParIdentifiant(venteChoisie)) {
      venteChoisie = null;
      cacherAnnulation();
    }

    marquerChoix();
  }

  function cacherAnnulation() {
    element("session_annulation").hidden = true;
    element("session_annulation_code").value = "";
  }

  // La croix et la touche Suppr ne suppriment rien a elles seules : elles
  // ouvrent une demande, qui exige un code. Une vente n est pas une ligne de
  // panier, elle est deja de l argent compte.
  function demanderAnnulation(identifiant) {
    if (!annulationActive) {
      global.notifier.neutre(
        "Annulation des ventes desactivee : le gerant doit l autoriser dans Parametrage."
      );
      return;
    }

    const vente = venteParIdentifiant(identifiant);

    if (!vente) {
      return;
    }

    element("session_annulation_question").textContent =
      "Annuler la vente no " +
      vente.numero_ticket +
      " de " +
      global.format.monnaie(vente.total_centimes) +
      ", encaissee par " +
      vente.caissier +
      " le " +
      vente.horodatage +
      " ?";
    element("session_annulation").hidden = false;
    element("session_annulation_code").value = "";
    element("session_annulation_code").focus();
  }

  async function confirmerAnnulation() {
    if (venteChoisie === null) {
      return;
    }

    try {
      const trace = await global.pont.appeler("annuler_vente", {
        vente: venteChoisie,
        code: element("session_annulation_code").value
      });

      venteChoisie = null;
      cacherAnnulation();
      global.notifier.succes(
        "Vente no " +
          trace.numero_ticket +
          " annulee par " +
          trace.annulee_par +
          ". Stock rendu, trace conservee dans Rapports."
      );

      await rafraichir();
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      element("session_annulation_code").value = "";
      element("session_annulation_code").focus();
      global.notifier.incident(incident);
    }
  }

  function surTouche(evenement) {
    if (!active || evenement.defaultPrevented) {
      return;
    }

    if (evenement.key !== "Delete" || venteChoisie === null) {
      return;
    }

    const cible = evenement.target;

    if (cible && typeof cible.closest === "function" &&
        cible.closest("input, select, textarea")) {
      return;
    }

    evenement.preventDefault();
    demanderAnnulation(venteChoisie);
  }

  async function ouvrir() {
    if (!caissierChoisi) {
      global.notifier.alerte("Choisis d abord un caissier.");
      return;
    }

    try {
      session = await global.pont.appeler("ouvrir_session_caissier", {
        caissier: caissierChoisi,
        code: element("session_code").value,
        fond: element("session_fond_saisie").value
      });

      element("session_code").value = "";
      afficherEtat();
      global.notifier.succes("Session ouverte pour " + session.caissier_nom + ".");
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function fermer() {
    try {
      const fermee = await global.pont.appeler("fermer_session_caissier", {
        code: element("session_code_fermeture").value
      });

      element("session_code_fermeture").value = "";
      session = null;
      afficherEtat();
      global.notifier.succes(
        "Session de " +
          fermee.caissier_nom +
          " fermee. Encaisse : " +
          global.format.monnaie(fermee.total_centimes)
      );
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function rafraichir() {
    try {
      const [etat, actifs, journal] = await Promise.all([
        global.pont.appeler("recuperer_session_courante"),
        global.pont.appeler("lister_caissiers_actifs"),
        global.pont.appeler("recuperer_journal_caisse")
      ]);

      session = etat.ouverte ? etat.session : null;
      caissiers = actifs;
      ventes = journal.ventes;
      annulationActive = journal.annulation_active;

      if (!caissiers.some(function (caissier) {
          return caissier.identifiant === caissierChoisi;
        })) {
        caissierChoisi = caissiers.length === 1 ? caissiers[0].identifiant : null;
      }

      afficherEtat();
      afficherCaissiers();
      afficherJournal();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("session_ouvrir").addEventListener("click", ouvrir);
    element("session_fermer").addEventListener("click", fermer);

    element("session_code").addEventListener("keydown", function (evenement) {
      if (evenement.key === "Enter") {
        ouvrir();
      }
    });

    element("session_code_fermeture").addEventListener("keydown", function (evenement) {
      if (evenement.key === "Enter") {
        fermer();
      }
    });

    element("session_annulation_valider").addEventListener("click", confirmerAnnulation);
    element("session_annulation_renoncer").addEventListener("click", function () {
      cacherAnnulation();
    });

    element("session_annulation_code").addEventListener("keydown", function (evenement) {
      if (evenement.key === "Enter") {
        evenement.preventDefault();
        confirmerAnnulation();
      }
    });

    document.addEventListener("keydown", surTouche);
  }

  global.vueSession = {
    initialiser: initialiser,
    rafraichir: rafraichir,
    entrer: function () {
      active = true;
      rafraichir();
    },
    sortir: function () {
      active = false;
      venteChoisie = null;
      cacherAnnulation();
    }
  };
})(window);
