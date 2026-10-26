(function (global) {
  "use strict";

  const element = global.format.element;

  let texteAffiche = "";

  function bouton(texte, classe, action) {
    const noeud = global.format.creer("button", classe, texte);

    noeud.type = "button";
    noeud.addEventListener("click", action);

    return noeud;
  }

  // Section du parametrage : on n y entre qu avec le code du gerant, le corps
  // suit donc simplement l etat de l acces.
  function appliquerVisibilite() {
    element("rapports_corps").hidden = !global.acces.actif();
  }

  function afficherMesures(rapport) {
    element("rapports_nombre_ventes").textContent = String(rapport.donnees.nombre_ventes);
    element("rapports_total").textContent = global.format.monnaie(rapport.donnees.total_centimes);
    element("rapports_grand_total").textContent = global.format.monnaie(
      rapport.donnees.grand_total_centimes
    );
    element("rapports_razs").textContent = String(rapport.donnees.remises_a_zero);
    element("rapports_numero_ticket").textContent = String(rapport.donnees.dernier_ticket);
    element("rapports_alerte_session").hidden = !rapport.session_ouverte;
    element("rapports_export_alerte").hidden = !rapport.session_ouverte;
  }

  function afficherTexte(texte) {
    texteAffiche = texte || "";

    const zone = element("rapports_texte_z");

    zone.textContent = texteAffiche;
    zone.hidden = !texteAffiche;
    element("rapports_imprimer").hidden = !texteAffiche;
  }

  function afficherJournal(ventes) {
    const corps = element("rapports_journal");

    global.format.vider(corps);

    if (!ventes.length) {
      corps.appendChild(global.format.ligneVide(7, "Aucune vente en memoire."));
      return;
    }

    ventes.forEach(function (vente) {
      const ligne = global.format.creer("tr");

      ligne.appendChild(global.format.cellule("no " + vente.numero_ticket));
      ligne.appendChild(global.format.cellule(vente.horodatage));
      ligne.appendChild(global.format.cellule(vente.caissier));
      ligne.appendChild(global.format.cellule(vente.articles || "--"));
      ligne.appendChild(global.format.cellule(vente.nombre_articles, "tableau__chiffre"));
      ligne.appendChild(
        global.format.cellule(global.format.monnaie(vente.total_centimes), "tableau__chiffre")
      );

      const actions = global.format.creer("td");
      const groupe = global.format.creer("div", "tableau__actions");

      groupe.appendChild(
        bouton("Ticket", "bouton bouton--petit", function () {
          relire(vente.identifiant);
        })
      );

      actions.appendChild(groupe);
      ligne.appendChild(actions);
      corps.appendChild(ligne);
    });
  }

  function afficherTotaux(totaux) {
    const corps = element("rapports_totaux");

    global.format.vider(corps);

    if (!totaux.length) {
      corps.appendChild(global.format.ligneVide(3, "Aucun encaissement."));
      return;
    }

    totaux.forEach(function (total) {
      const ligne = global.format.creer("tr");

      ligne.appendChild(global.format.cellule(total.libelle));
      ligne.appendChild(global.format.cellule(total.quantite, "tableau__chiffre"));
      ligne.appendChild(
        global.format.cellule(global.format.monnaie(total.total_centimes), "tableau__chiffre")
      );

      corps.appendChild(ligne);
    });
  }

  // Trace des annulations de la periode. Une vente annulee quitte le journal :
  // c est le seul endroit ou le trou dans la suite des tickets s explique.
  function afficherAnnulations(annulations) {
    const corps = element("rapports_annulations");

    global.format.vider(corps);

    if (!annulations.length) {
      corps.appendChild(global.format.ligneVide(6, "Aucune vente annulee."));
      return;
    }

    annulations.forEach(function (annulation) {
      const ligne = global.format.creer("tr");

      ligne.appendChild(global.format.cellule("no " + annulation.numero_ticket));
      ligne.appendChild(global.format.cellule(annulation.encaissee_le));
      ligne.appendChild(global.format.cellule(annulation.caissier));
      ligne.appendChild(global.format.cellule(annulation.annulee_le));
      ligne.appendChild(global.format.cellule(annulation.annulee_par));
      ligne.appendChild(
        global.format.cellule(
          global.format.monnaie(annulation.total_centimes),
          "tableau__chiffre"
        )
      );

      corps.appendChild(ligne);
    });
  }

  function afficherArchives(rapports) {
    const corps = element("rapports_archives");

    global.format.vider(corps);

    if (!rapports.length) {
      corps.appendChild(global.format.ligneVide(5, "Aucun rapport Z edite."));
      return;
    }

    rapports.forEach(function (rapport) {
      const ligne = global.format.creer("tr");

      ligne.appendChild(global.format.cellule("Z " + rapport.numero));
      ligne.appendChild(global.format.cellule(rapport.genere_le));
      ligne.appendChild(global.format.cellule(rapport.nombre_ventes, "tableau__chiffre"));
      ligne.appendChild(
        global.format.cellule(global.format.monnaie(rapport.total_centimes), "tableau__chiffre")
      );

      const actions = global.format.creer("td");
      const groupe = global.format.creer("div", "tableau__actions");

      groupe.appendChild(
        bouton("Relire", "bouton bouton--petit", function () {
          afficherTexte(rapport.texte);
          element("rapports_cloture").hidden = true;
        })
      );

      actions.appendChild(groupe);
      ligne.appendChild(actions);
      corps.appendChild(ligne);
    });
  }

  function remplirCaissiers(caissiers) {
    const liste = element("rapports_filtre_caissier");
    const choisi = liste.value;

    global.format.vider(liste);
    liste.appendChild(new Option("Tous les caissiers", ""));

    caissiers.forEach(function (caissier) {
      liste.appendChild(new Option(caissier.nom, String(caissier.identifiant)));
    });

    liste.value = choisi;
  }

  async function relire(vente) {
    try {
      const ticket = await global.pont.appeler("relire_ticket", {
        jeton: global.acces.exiger(),
        vente: vente
      });

      afficherTexte(ticket.texte);
      element("rapports_cloture").hidden = true;
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function previsualiser() {
    try {
      const rapport = await global.pont.appeler("previsualiser_rapport_z", {
        jeton: global.acces.exiger()
      });

      afficherMesures(rapport);
      afficherTexte(rapport.texte);

      element("rapports_cloture").hidden = rapport.donnees.nombre_ventes === 0;
      element("rapports_confirmation").checked = false;
      element("rapports_cloturer").disabled = true;
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function cloturer() {
    try {
      const rapport = await global.pont.appeler("cloturer_rapport_z", {
        jeton: global.acces.exiger()
      });

      afficherTexte(rapport.texte);
      element("rapports_cloture").hidden = true;
      global.notifier.succes(
        "Rapport Z no " + rapport.donnees.numero + " edite. Memoire des ventes effacee."
      );

      global.impression.imprimer(rapport.texte);

      await rafraichir();
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  // Export du detail puis remise a zero. Le noyau ecrit le fichier avant
  // d effacer : un fichier absent veut dire une periode intacte.
  async function exporterEtVider() {
    const resultat = element("rapports_export_resultat");

    try {
      const sortie = await global.pont.appeler("exporter_ventes_et_vider", {
        jeton: global.acces.exiger()
      });

      afficherTexte(sortie.rapport.texte);
      element("rapports_cloture").hidden = true;
      element("rapports_export_confirmation").checked = false;
      element("rapports_exporter_vider").disabled = true;

      resultat.textContent =
        sortie.lignes +
        " ligne(s) d articles ecrite(s) dans " +
        sortie.chemin +
        " - memoire des ventes effacee (rapport Z no " +
        sortie.rapport.donnees.numero +
        ").";
      resultat.hidden = false;

      global.notifier.succes("Detail exporte dans " + sortie.nom + ". Memoire des ventes effacee.");

      await rafraichir();
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      resultat.hidden = true;
      global.notifier.incident(incident);
    }
  }

  async function rafraichir() {
    if (!global.acces.actif()) {
      appliquerVisibilite();
      return;
    }

    appliquerVisibilite();

    try {
      const jeton = global.acces.exiger();
      const caissierChoisi = element("rapports_filtre_caissier").value;

      const [rapport, ventes, totaux, caissiers, archives, annulations] = await Promise.all([
        global.pont.appeler("previsualiser_rapport_z", { jeton: jeton }),
        global.pont.appeler("lister_ventes", {
          jeton: jeton,
          caissier: caissierChoisi ? global.format.entier(caissierChoisi, null) : null
        }),
        global.pont.appeler("recuperer_totaux_caissiers", { jeton: jeton }),
        global.pont.appeler("lister_caissiers", { jeton: jeton }),
        global.pont.appeler("lister_rapports_z", { jeton: jeton }),
        global.pont.appeler("lister_ventes_annulees", { jeton: jeton })
      ]);

      afficherMesures(rapport);
      afficherJournal(ventes);
      afficherTotaux(totaux);
      remplirCaissiers(caissiers);
      afficherArchives(archives);
      afficherAnnulations(annulations);
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("rapports_previsualiser").addEventListener("click", previsualiser);
    element("rapports_cloturer").addEventListener("click", cloturer);
    element("rapports_rafraichir").addEventListener("click", rafraichir);
    element("rapports_filtre_caissier").addEventListener("change", rafraichir);

    element("rapports_confirmation").addEventListener("change", function (evenement) {
      element("rapports_cloturer").disabled = !evenement.target.checked;
    });

    element("rapports_exporter_vider").addEventListener("click", exporterEtVider);

    element("rapports_export_confirmation").addEventListener("change", function (evenement) {
      element("rapports_exporter_vider").disabled = !evenement.target.checked;
    });

    element("rapports_imprimer").addEventListener("click", function () {
      global.impression.imprimer(texteAffiche);
    });

    global.acces.surChangement(appliquerVisibilite);
  }

  global.vueRapports = {
    initialiser: initialiser,
    rafraichir: rafraichir,
    entrer: rafraichir
  };
})(window);
