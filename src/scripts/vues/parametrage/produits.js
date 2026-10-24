(function (global) {
  "use strict";

  const element = global.format.element;

  // Valeur du champ stock qui dispense le produit de tout decompte, et valeur
  // par defaut d une fiche neuve : on vend d abord, on compte apres.
  const STOCK_SANS_SUIVI = "NON";

  let liste = [];
  let rayons = [];
  let enEdition = null;

  function reinitialiserFormulaire() {
    enEdition = null;
    element("produit_titre_formulaire").textContent = "Nouveau produit";
    element("produit_libelle").value = "";
    element("produit_code").value = "";
    element("produit_prix").value = "";
    element("produit_stock").value = STOCK_SANS_SUIVI;
    element("produit_actif").checked = true;
    element("produit_enregistrer").textContent = "Creer";
    element("produit_annuler").hidden = true;
    afficher();
  }

  function editer(produit) {
    enEdition = produit.identifiant;
    element("produit_titre_formulaire").textContent = "Modifier " + produit.libelle;
    element("produit_libelle").value = produit.libelle;
    element("produit_code").value = produit.code_barres || "";
    element("produit_prix").value = global.format.monnaie(produit.prix_centimes).replace(/ /g, "");
    element("produit_stock").value = produit.suivi_stock
      ? String(produit.stock)
      : STOCK_SANS_SUIVI;
    element("produit_actif").checked = produit.actif;
    element("produit_rayon").value = String(produit.rayon);
    element("produit_enregistrer").textContent = "Enregistrer";
    element("produit_annuler").hidden = false;
    afficher();
    element("produit_libelle").focus();
  }

  function bouton(texte, classe, action) {
    const noeud = global.format.creer("button", classe, texte);

    noeud.type = "button";
    noeud.addEventListener("click", action);

    return noeud;
  }

  function afficher() {
    const corps = element("produits_liste");

    global.format.vider(corps);

    if (!liste.length) {
      corps.appendChild(global.format.ligneVide(6, "Aucun produit pour ce filtre."));
      return;
    }

    liste.forEach(function (produit) {
      const ligne = global.format.creer("tr");

      if (enEdition === produit.identifiant) {
        ligne.dataset.selectionne = "oui";
      }

      ligne.appendChild(global.format.cellule(produit.libelle));
      ligne.appendChild(global.format.cellule(produit.rayon_libelle));
      ligne.appendChild(global.format.cellule(produit.code_barres || "--"));
      ligne.appendChild(
        global.format.cellule(global.format.monnaie(produit.prix_centimes), "tableau__chiffre")
      );
      ligne.appendChild(
        global.format.cellule(
          produit.suivi_stock ? produit.stock : STOCK_SANS_SUIVI,
          "tableau__chiffre"
        )
      );

      const actions = global.format.creer("td");
      const groupe = global.format.creer("div", "tableau__actions");

      groupe.appendChild(
        bouton("Modifier", "bouton bouton--petit", function () {
          editer(produit);
        })
      );
      groupe.appendChild(
        global.format.boutonConfirme(
          "Supprimer",
          "bouton bouton--petit bouton--danger",
          function () {
            supprimer(produit);
          }
        )
      );

      actions.appendChild(groupe);
      ligne.appendChild(actions);
      corps.appendChild(ligne);
    });
  }

  function remplirListeRayons() {
    const liste_rayons = element("produit_rayon");
    const choisi = liste_rayons.value;

    global.format.vider(liste_rayons);

    rayons.forEach(function (rayon) {
      liste_rayons.appendChild(new Option(rayon.libelle, String(rayon.identifiant)));
    });

    if (choisi) {
      liste_rayons.value = choisi;
    }
  }

  async function rafraichirRayons() {
    try {
      rayons = await global.pont.appeler("lister_rayons", { inclureInactifs: true });
      remplirListeRayons();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function rafraichir() {
    try {
      liste = await global.pont.appeler("lister_produits", {
        recherche: element("produits_recherche").value,
        inclureInactifs: true
      });

      afficher();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function enregistrer() {
    const rayon = global.format.entier(element("produit_rayon").value, 0);

    if (!rayon) {
      global.notifier.alerte("Cree d abord un rayon : tout produit appartient a un rayon.");
      return;
    }

    const charge = {
      jeton: global.acces.exiger(),
      rayon: rayon,
      libelle: element("produit_libelle").value,
      codeBarres: element("produit_code").value,
      prix: element("produit_prix").value,
      stock: element("produit_stock").value
    };

    try {
      if (enEdition === null) {
        await global.pont.appeler("creer_produit", charge);
        global.notifier.succes("Produit cree.");
      } else {
        charge.identifiant = enEdition;
        charge.actif = element("produit_actif").checked;

        await global.pont.appeler("modifier_produit", charge);
        global.notifier.succes("Produit mis a jour.");
      }

      reinitialiserFormulaire();
      await rafraichir();
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function supprimer(produit) {
    try {
      await global.pont.appeler("supprimer_produit", {
        jeton: global.acces.exiger(),
        identifiant: produit.identifiant
      });

      global.notifier.succes("Produit supprime.");

      if (enEdition === produit.identifiant) {
        reinitialiserFormulaire();
      }

      await rafraichir();
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  // Une etiquette scannee pendant la saisie remplit le champ code-barres.
  function surScan(charge) {
    element("produit_code").value = charge.code;
    global.notifier.neutre("Code " + charge.code + " recopie dans la fiche produit.");
  }

  function initialiser() {
    element("produit_enregistrer").addEventListener("click", enregistrer);
    element("produit_annuler").addEventListener("click", reinitialiserFormulaire);
    element("produits_recherche").addEventListener("input", rafraichir);
  }

  global.reglageProduits = {
    initialiser: initialiser,
    rafraichir: rafraichir,
    rafraichirRayons: rafraichirRayons,
    surScan: surScan
  };
})(window);
