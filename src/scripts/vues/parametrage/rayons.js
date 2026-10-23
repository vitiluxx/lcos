(function (global) {
  "use strict";

  const element = global.format.element;

  let liste = [];
  let enEdition = null;

  function reinitialiserFormulaire() {
    enEdition = null;
    element("rayon_titre_formulaire").textContent = "Nouveau rayon";
    element("rayon_libelle").value = "";
    element("rayon_ordre").value = "0";
    element("rayon_actif").checked = true;
    element("rayon_enregistrer").textContent = "Creer";
    element("rayon_annuler").hidden = true;
    afficher();
  }

  function editer(rayon) {
    enEdition = rayon.identifiant;
    element("rayon_titre_formulaire").textContent = "Modifier " + rayon.libelle;
    element("rayon_libelle").value = rayon.libelle;
    element("rayon_ordre").value = String(rayon.ordre);
    element("rayon_actif").checked = rayon.actif;
    element("rayon_enregistrer").textContent = "Enregistrer";
    element("rayon_annuler").hidden = false;
    afficher();
    element("rayon_libelle").focus();
  }

  function bouton(texte, classe, action) {
    const noeud = global.format.creer("button", classe, texte);

    noeud.type = "button";
    noeud.addEventListener("click", action);

    return noeud;
  }

  function afficher() {
    const corps = element("rayons_liste");

    global.format.vider(corps);

    if (!liste.length) {
      corps.appendChild(global.format.ligneVide(5, "Aucun rayon : commence par en creer un."));
      return;
    }

    liste.forEach(function (rayon) {
      const ligne = global.format.creer("tr");

      if (enEdition === rayon.identifiant) {
        ligne.dataset.selectionne = "oui";
      }

      ligne.appendChild(global.format.cellule(rayon.libelle));
      ligne.appendChild(global.format.cellule(rayon.ordre, "tableau__chiffre"));

      const etat = global.format.creer("td");
      etat.appendChild(
        global.format.creer(
          "span",
          rayon.actif ? "badge badge--succes" : "badge badge--attente",
          rayon.actif ? "actif" : "masque"
        )
      );
      ligne.appendChild(etat);

      ligne.appendChild(global.format.cellule(rayon.nombre_produits, "tableau__chiffre"));

      const actions = global.format.creer("td");
      const groupe = global.format.creer("div", "tableau__actions");

      groupe.appendChild(
        bouton("Modifier", "bouton bouton--petit", function () {
          editer(rayon);
        })
      );
      groupe.appendChild(
        global.format.boutonConfirme(
          "Supprimer",
          "bouton bouton--petit bouton--danger",
          function () {
            supprimer(rayon);
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
      liste = await global.pont.appeler("lister_rayons", { inclureInactifs: true });
      afficher();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function enregistrer() {
    const libelle = element("rayon_libelle").value;
    const ordre = global.format.entier(element("rayon_ordre").value, 0);

    try {
      if (enEdition === null) {
        await global.pont.appeler("creer_rayon", {
          jeton: global.acces.exiger(),
          libelle: libelle,
          ordre: ordre
        });

        global.notifier.succes("Rayon cree.");
      } else {
        await global.pont.appeler("modifier_rayon", {
          jeton: global.acces.exiger(),
          identifiant: enEdition,
          libelle: libelle,
          ordre: ordre,
          actif: element("rayon_actif").checked
        });

        global.notifier.succes("Rayon mis a jour.");
      }

      reinitialiserFormulaire();
      await rafraichir();
      global.reglageProduits.rafraichirRayons();
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function supprimer(rayon) {
    try {
      await global.pont.appeler("supprimer_rayon", {
        jeton: global.acces.exiger(),
        identifiant: rayon.identifiant
      });

      global.notifier.succes("Rayon supprime.");

      if (enEdition === rayon.identifiant) {
        reinitialiserFormulaire();
      }

      await rafraichir();
      global.reglageProduits.rafraichirRayons();
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("rayon_enregistrer").addEventListener("click", enregistrer);
    element("rayon_annuler").addEventListener("click", reinitialiserFormulaire);
  }

  global.reglageRayons = {
    initialiser: initialiser,
    rafraichir: rafraichir
  };
})(window);
