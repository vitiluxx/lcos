/*
 * Vue d encaissement. Le panier ne porte que des identifiants et des
 * quantites : les prix affiches viennent du catalogue charge, et c est le
 * noyau Rust qui refait le chiffrage au moment de l encaissement.
 */
(function (global) {
  "use strict";

  const CADENCE_MACHINE_MS = 40;
  const DELAI_STABILITE_MS = 300;
  const CHIFFRES_ENTIERS_MAXIMAUX = 8;

  const panier = [];

  let produits = [];
  let reglages = null;
  let session = null;
  let montantSaisi = "";
  let montantStable = "";
  let minuterieStable = null;
  let dernierInstantTouche = 0;
  let encaissementEnCours = false;
  let active = false;

  const element = global.format.element;

  function totalCentimes() {
    return panier.reduce(function (somme, ligne) {
      return somme + ligne.prix_centimes * ligne.quantite;
    }, 0);
  }

  function recuCentimes() {
    if (!montantSaisi) {
      return 0;
    }

    const tranches = montantSaisi.split(",");
    const unites = global.format.entier(tranches[0] || "0", 0);
    const decimales = (tranches[1] || "").padEnd(2, "0").slice(0, 2);

    return unites * 100 + global.format.entier(decimales, 0);
  }

  function afficherMontant() {
    element("caisse_montant").value = montantSaisi || "0,00";
  }

  function afficherTotaux() {
    const total = totalCentimes();
    const recu = recuCentimes();
    const rendu = recu - total;
    const cadre = element("caisse_rendu_cadre");

    element("caisse_total").textContent = global.format.monnaie(total);
    element("caisse_rendu").textContent = global.format.monnaie(Math.abs(rendu));

    const insuffisant = montantSaisi !== "" && rendu < 0;

    cadre.className = insuffisant
      ? "paiement__rendu paiement__rendu--insuffisant"
      : "paiement__rendu";
    cadre.firstElementChild.textContent = insuffisant ? "Manque" : "A rendre";

    element("caisse_encaisser").disabled =
      encaissementEnCours || panier.length === 0 || recu < total;
  }

  function afficherPanier() {
    const corps = element("caisse_panier");

    global.format.vider(corps);

    if (!panier.length) {
      corps.appendChild(
        global.format.ligneVide(5, "Panier vide : scanne un article ou touche la grille.")
      );
      afficherTotaux();
      return;
    }

    panier.forEach(function (ligne, rang) {
      const tr = global.format.creer("tr");

      tr.appendChild(global.format.cellule(ligne.libelle));
      tr.appendChild(
        global.format.cellule(global.format.monnaie(ligne.prix_centimes), "tableau__chiffre")
      );
      tr.appendChild(global.format.cellule(ligne.quantite, "tableau__chiffre"));
      tr.appendChild(
        global.format.cellule(
          global.format.monnaie(ligne.prix_centimes * ligne.quantite),
          "tableau__chiffre"
        )
      );

      const actions = global.format.creer("td");
      const groupe = global.format.creer("div", "tableau__actions");

      groupe.appendChild(bouton("-", "bouton bouton--petit", function () {
        changerQuantite(rang, -1);
      }));
      groupe.appendChild(bouton("+", "bouton bouton--petit", function () {
        changerQuantite(rang, 1);
      }));
      groupe.appendChild(bouton("x", "bouton bouton--petit bouton--danger", function () {
        retirerLigne(rang);
      }));

      actions.appendChild(groupe);
      tr.appendChild(actions);
      corps.appendChild(tr);
    });

    afficherTotaux();
  }

  function bouton(texte, classe, action) {
    const noeud = global.format.creer("button", classe, texte);

    noeud.type = "button";
    noeud.addEventListener("click", action);

    return noeud;
  }

  function ajouterProduit(produit, quantite) {
    const pas = quantite || 1;
    const existante = panier.find(function (ligne) {
      return ligne.produit === produit.identifiant;
    });

    if (existante) {
      existante.quantite += pas;
    } else {
      panier.push({
        produit: produit.identifiant,
        libelle: produit.libelle,
        prix_centimes: produit.prix_centimes,
        quantite: pas
      });
    }

    afficherPanier();
  }

  function changerQuantite(rang, pas) {
    const ligne = panier[rang];

    if (!ligne) {
      return;
    }

    ligne.quantite += pas;

    if (ligne.quantite <= 0) {
      panier.splice(rang, 1);
    }

    afficherPanier();
  }

  function retirerLigne(rang) {
    panier.splice(rang, 1);
    afficherPanier();
  }

  function viderPanier() {
    panier.length = 0;
    montantSaisi = "";
    montantStable = "";
    afficherMontant();
    afficherPanier();
  }

  function afficherGrille() {
    const grille = element("caisse_grille");
    const rayonChoisi = element("caisse_filtre_rayon").value;
    const recherche = element("caisse_recherche").value.trim().toLowerCase();

    global.format.vider(grille);

    const retenus = produits.filter(function (produit) {
      if (rayonChoisi && String(produit.rayon) !== rayonChoisi) {
        return false;
      }

      if (!recherche) {
        return true;
      }

      const code = produit.code_barres || "";

      return (
        produit.libelle.toLowerCase().indexOf(recherche) !== -1 ||
        code.indexOf(recherche) !== -1
      );
    });

    if (!retenus.length) {
      grille.appendChild(
        global.format.creer("p", "caisse__aide", "Aucun produit : cree ton catalogue dans Parametrage.")
      );
      return;
    }

    retenus.forEach(function (produit) {
      const pastille = global.format.creer("button", "pastille_produit");

      pastille.type = "button";
      pastille.appendChild(
        global.format.creer("span", "pastille_produit__libelle", produit.libelle)
      );
      pastille.appendChild(
        global.format.creer(
          "span",
          "pastille_produit__prix",
          global.format.monnaie(produit.prix_centimes)
        )
      );
      pastille.appendChild(
        global.format.creer("span", "pastille_produit__rayon", produit.rayon_libelle)
      );
      pastille.addEventListener("click", function () {
        ajouterProduit(produit, 1);
      });

      grille.appendChild(pastille);
    });
  }

  function remplirRayons(rayons) {
    const liste = element("caisse_filtre_rayon");
    const choisi = liste.value;

    global.format.vider(liste);
    liste.appendChild(new Option("Tous les rayons", ""));

    rayons.forEach(function (rayon) {
      liste.appendChild(new Option(rayon.libelle, String(rayon.identifiant)));
    });

    liste.value = choisi;
  }

  function stabiliserMontant() {
    if (minuterieStable !== null) {
      clearTimeout(minuterieStable);
    }

    minuterieStable = setTimeout(function () {
      montantStable = montantSaisi;
    }, DELAI_STABILITE_MS);
  }

  function ajouterAuMontant(caracteres) {
    let suite = montantSaisi;

    for (const caractere of caracteres) {
      if (caractere === ",") {
        if (suite.indexOf(",") !== -1) {
          continue;
        }

        suite = (suite || "0") + ",";
        continue;
      }

      const tranches = suite.split(",");

      if (tranches.length > 1) {
        if (tranches[1].length >= 2) {
          continue;
        }
      } else if (tranches[0].length >= CHIFFRES_ENTIERS_MAXIMAUX) {
        continue;
      }

      suite += caractere;
    }

    montantSaisi = suite;
    afficherMontant();
    afficherTotaux();
    stabiliserMontant();
  }

  function retirerDuMontant() {
    montantSaisi = montantSaisi.slice(0, -1);
    afficherMontant();
    afficherTotaux();
    stabiliserMontant();
  }

  function montantExact() {
    const total = totalCentimes();

    montantSaisi = global.format.monnaie(total).replace(/ /g, "");
    montantStable = montantSaisi;
    afficherMontant();
    afficherTotaux();
  }

  function effacerMontant() {
    montantSaisi = "";
    montantStable = "";
    afficherMontant();
    afficherTotaux();
  }

  async function encaisser() {
    if (encaissementEnCours || !panier.length) {
      return;
    }

    encaissementEnCours = true;
    afficherTotaux();

    try {
      const ticket = await global.pont.appeler("encaisser", {
        lignes: panier.map(function (ligne) {
          return { produit: ligne.produit, quantite: ligne.quantite };
        }),
        recu: montantSaisi || "0"
      });

      afficherTicket(ticket);
      global.notifier.succes(
        "Ticket no " +
          ticket.numero_ticket +
          " encaisse. A rendre : " +
          global.format.monnaie(ticket.rendu_centimes)
      );

      viderPanier();
      await rafraichirCatalogue();

      if (global.vueSession) {
        global.vueSession.rafraichir();
      }
    } catch (incident) {
      global.notifier.incident(incident);
    } finally {
      encaissementEnCours = false;
      afficherTotaux();
    }
  }

  function afficherTicket(ticket) {
    const panneau = element("caisse_ticket_panneau");

    element("caisse_ticket_numero").textContent = String(ticket.numero_ticket);
    element("caisse_ticket_texte").textContent = ticket.texte;
    panneau.hidden = !ticket.ticket_actif;

    if (ticket.ticket_actif && ticket.impression_auto) {
      global.impression.imprimer(ticket.texte);
    }
  }

  async function surScan(charge) {
    if (!session) {
      global.notifier.alerte("Aucune session ouverte : impossible d encaisser ce scan.");
      return;
    }

    // Les premiers caracteres du code ont pu tomber dans la zone montant :
    // on restitue la derniere valeur stable avant de traiter le code.
    montantSaisi = montantStable;
    afficherMontant();

    element("caisse_dernier_code").textContent = charge.code;

    try {
      const resultat = await global.pont.appeler("trouver_produit_par_code", {
        code: charge.code
      });

      if (!resultat.trouve) {
        global.notifier.alerte(
          "Code " + resultat.code + " inconnu : cree le produit dans Parametrage."
        );
        afficherTotaux();
        return;
      }

      ajouterProduit(resultat.produit, 1);
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function dansUnChamp(cible) {
    if (!cible || typeof cible.closest !== "function") {
      return false;
    }

    return Boolean(cible.closest("input, select, textarea"));
  }

  function surTouche(evenement) {
    if (!active || !session || evenement.defaultPrevented) {
      return;
    }

    if (evenement.ctrlKey || evenement.altKey || evenement.metaKey) {
      return;
    }

    if (dansUnChamp(evenement.target)) {
      return;
    }

    const touche = evenement.key;

    if (touche === "Enter") {
      evenement.preventDefault();
      encaisser();
      return;
    }

    if (touche === "Escape") {
      evenement.preventDefault();
      viderPanier();
      global.notifier.neutre("Vente annulee.");
      return;
    }

    if (touche === "Backspace") {
      evenement.preventDefault();
      retirerDuMontant();
      return;
    }

    const estChiffre = touche.length === 1 && touche >= "0" && touche <= "9";
    const estVirgule = touche === "," || touche === ".";

    if (!estChiffre && !estVirgule) {
      return;
    }

    const instant = Date.now();
    const ecart = instant - dernierInstantTouche;

    dernierInstantTouche = instant;

    // Rafale trop rapide pour une main humaine : c est la douchette qui tape.
    if (ecart < CADENCE_MACHINE_MS) {
      return;
    }

    evenement.preventDefault();
    ajouterAuMontant(estVirgule ? "," : touche);
  }

  async function rafraichirCatalogue() {
    const [listeRayons, listeProduits] = await Promise.all([
      global.pont.appeler("lister_rayons", {}),
      global.pont.appeler("lister_produits", {})
    ]);

    produits = listeProduits;
    remplirRayons(listeRayons);
    afficherGrille();
  }

  function appliquerSession(etat) {
    session = etat && etat.ouverte ? etat.session : null;

    element("caisse_verrou").hidden = Boolean(session);
    element("caisse_corps").hidden = !session;

    if (!session) {
      viderPanier();
    }
  }

  function appliquerReglages(nouveaux) {
    reglages = nouveaux;

    element("caisse_etat_ticket").textContent =
      "ticket : " + (nouveaux.ticket_actif ? "actif" : "desactive");
    element("caisse_etat_ticket").className = nouveaux.ticket_actif
      ? "badge badge--succes"
      : "badge badge--attente";
  }

  async function rafraichir() {
    try {
      const [etatSession, parametrage] = await Promise.all([
        global.pont.appeler("recuperer_session_courante"),
        global.pont.appeler("recuperer_reglages")
      ]);

      appliquerSession(etatSession);
      appliquerReglages(parametrage);

      await rafraichirCatalogue();
      afficherPanier();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function basculerTicket() {
    if (!global.acces.actif()) {
      global.notifier.neutre("Code du gerant requis pour changer le reglage du ticket.");
      global.application.allerA("parametrage");
      return;
    }

    try {
      appliquerReglages(
        await global.pont.appeler("basculer_ticket", {
          jeton: global.acces.jeton(),
          actif: !(reglages && reglages.ticket_actif)
        })
      );

      global.notifier.succes(
        reglages.ticket_actif ? "Ticket de caisse active." : "Ticket de caisse desactive."
      );
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("caisse_etat_ticket").addEventListener("click", basculerTicket);
    element("caisse_recherche").addEventListener("input", afficherGrille);
    element("caisse_filtre_rayon").addEventListener("change", afficherGrille);
    element("caisse_encaisser").addEventListener("click", encaisser);
    element("caisse_annuler").addEventListener("click", function () {
      viderPanier();
      global.notifier.neutre("Vente annulee.");
    });
    element("caisse_imprimer").addEventListener("click", function () {
      global.impression.imprimer(element("caisse_ticket_texte").textContent);
    });
    element("caisse_fermer_ticket").addEventListener("click", function () {
      element("caisse_ticket_panneau").hidden = true;
    });

    document.querySelectorAll("[data-touche]").forEach(function (touche) {
      touche.addEventListener("click", function () {
        const valeur = touche.dataset.touche;

        if (valeur === "effacer") {
          effacerMontant();
        } else if (valeur === "retour") {
          retirerDuMontant();
        } else if (valeur === "exact") {
          montantExact();
        } else {
          ajouterAuMontant(valeur);
        }
      });
    });

    document.addEventListener("keydown", surTouche);
    afficherMontant();
    afficherPanier();
  }

  global.vueCaisse = {
    initialiser: initialiser,
    rafraichir: rafraichir,
    surScan: surScan,
    entrer: function () {
      active = true;
      rafraichir();
    },
    sortir: function () {
      active = false;
    }
  };
})(window);
