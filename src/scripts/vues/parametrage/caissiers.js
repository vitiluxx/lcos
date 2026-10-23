(function (global) {
  "use strict";

  const element = global.format.element;

  let liste = [];
  let enEdition = null;

  function reinitialiserFormulaire() {
    enEdition = null;
    element("caissier_titre_formulaire").textContent = "Nouveau caissier";
    element("caissier_nom").value = "";
    element("caissier_code").value = "";
    element("caissier_actif").checked = true;
    element("caissier_enregistrer").textContent = "Creer";
    element("caissier_annuler").hidden = true;
    afficher();
  }

  function editer(caissier) {
    enEdition = caissier.identifiant;
    element("caissier_titre_formulaire").textContent = "Modifier " + caissier.nom;
    element("caissier_nom").value = caissier.nom;
    element("caissier_code").value = "";
    element("caissier_actif").checked = caissier.actif;
    element("caissier_enregistrer").textContent = "Enregistrer";
    element("caissier_annuler").hidden = false;
    afficher();
    element("caissier_nom").focus();
  }

  function bouton(texte, classe, action) {
    const noeud = global.format.creer("button", classe, texte);

    noeud.type = "button";
    noeud.addEventListener("click", action);

    return noeud;
  }

  function afficher() {
    const corps = element("caissiers_liste");

    global.format.vider(corps);

    if (!liste.length) {
      corps.appendChild(global.format.ligneVide(4, "Aucun caissier enregistre."));
      return;
    }

    liste.forEach(function (caissier) {
      const ligne = global.format.creer("tr");

      if (enEdition === caissier.identifiant) {
        ligne.dataset.selectionne = "oui";
      }

      ligne.appendChild(global.format.cellule(caissier.nom));

      const etat = global.format.creer("td");
      etat.appendChild(
        global.format.creer(
          "span",
          caissier.actif ? "badge badge--succes" : "badge badge--attente",
          caissier.actif ? "actif" : "desactive"
        )
      );
      ligne.appendChild(etat);

      ligne.appendChild(global.format.cellule(caissier.nombre_ventes, "tableau__chiffre"));

      const actions = global.format.creer("td");
      const groupe = global.format.creer("div", "tableau__actions");

      groupe.appendChild(
        bouton("Modifier", "bouton bouton--petit", function () {
          editer(caissier);
        })
      );
      groupe.appendChild(
        global.format.boutonConfirme(
          "Supprimer",
          "bouton bouton--petit bouton--danger",
          function () {
            supprimer(caissier);
          }
        )
      );

      actions.appendChild(groupe);
      ligne.appendChild(actions);
      corps.appendChild(ligne);
    });
  }

  async function rafraichir() {
    try {
      liste = await global.pont.appeler("lister_caissiers", {
        jeton: global.acces.exiger()
      });

      afficher();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function enregistrer() {
    const nom = element("caissier_nom").value;
    const code = element("caissier_code").value;

    try {
      if (enEdition === null) {
        await global.pont.appeler("creer_caissier", {
          jeton: global.acces.exiger(),
          nom: nom,
          code: code
        });

        global.notifier.succes("Caissier cree.");
      } else {
        await global.pont.appeler("modifier_caissier", {
          jeton: global.acces.exiger(),
          identifiant: enEdition,
          nom: nom,
          actif: element("caissier_actif").checked
        });

        if (code.trim()) {
          await global.pont.appeler("redefinir_code_caissier", {
            jeton: global.acces.exiger(),
            identifiant: enEdition,
            code: code
          });
        }

        global.notifier.succes("Caissier mis a jour.");
      }

      reinitialiserFormulaire();
      await rafraichir();
      global.application.rafraichirEtat();

      if (global.vueSession) {
        global.vueSession.rafraichir();
      }
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function supprimer(caissier) {
    try {
      await global.pont.appeler("supprimer_caissier", {
        jeton: global.acces.exiger(),
        identifiant: caissier.identifiant
      });

      global.notifier.succes("Caissier supprime.");

      if (enEdition === caissier.identifiant) {
        reinitialiserFormulaire();
      }

      await rafraichir();
      global.application.rafraichirEtat();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("caissier_enregistrer").addEventListener("click", enregistrer);
    element("caissier_annuler").addEventListener("click", reinitialiserFormulaire);
  }

  global.reglageCaissiers = {
    initialiser: initialiser,
    rafraichir: rafraichir
  };
})(window);
