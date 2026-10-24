/*
 * Vue d encaissement. Deux modes de saisie :
 *   - scan    : la douchette remplit le panier, le pave saisit la somme recue.
 *   - manuel  : le pave saisit le prix d un article, la touche + l ajoute a la
 *               sommation, la touche Entree arrete la sommation et donne la
 *               main au champ de la somme recue.
 * Le panier ne porte que des identifiants, des quantites et, pour les articles
 * libres, un prix : c est le noyau Rust qui refait le chiffrage.
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
  let mode = "scan";
  let phase = "paiement";
  let montantSaisi = "";
  let montantStable = "";
  let prixSaisi = "";
  let minuterieStable = null;
  let dernierInstantTouche = 0;
  let encaissementEnCours = false;
  let active = false;

  const element = global.format.element;

  function enSaisieDePrix() {
    return mode === "manuel" && phase === "saisie";
  }

  function totalCentimes() {
    return panier.reduce(function (somme, ligne) {
      return somme + ligne.prix_centimes * ligne.quantite;
    }, 0);
  }

  function enCentimes(texte) {
    return global.format.centimes(texte);
  }

  function recuCentimes() {
    return enCentimes(montantSaisi);
  }

  // Une frappe libre peut contenir n importe quoi : on ne garde que des
  // chiffres, une seule virgule et deux decimales au maximum.
  function normaliser(brut) {
    let resultat = "";

    for (const caractere of String(brut || "")) {
      const signe = caractere === "." ? "," : caractere;

      if (signe === ",") {
        if (!global.format.avecDecimales() || resultat.indexOf(",") !== -1) {
          continue;
        }

        resultat = (resultat || "0") + ",";
        continue;
      }

      if (signe < "0" || signe > "9") {
        continue;
      }

      const tranches = resultat.split(",");

      if (tranches.length > 1) {
        if (tranches[1].length >= 2) {
          continue;
        }
      } else if (tranches[0].length >= CHIFFRES_ENTIERS_MAXIMAUX) {
        continue;
      }

      resultat += signe;
    }

    return resultat;
  }

  function poser(champ, valeur) {
    if (champ.value !== valeur) {
      champ.value = valeur;
    }
  }

  function afficherMontant() {
    poser(element("caisse_montant"), montantSaisi || global.format.monnaie(0));
  }

  function afficherPrix() {
    poser(element("caisse_prix"), prixSaisi);
  }

  function etatPaiement(total, recu) {
    if (!panier.length || montantSaisi === "") {
      return "neutre";
    }

    if (recu < total) {
      return "insuffisant";
    }

    return recu === total ? "exact" : "rendu";
  }

  function afficherTotaux() {
    const total = totalCentimes();
    const recu = recuCentimes();
    const etat = etatPaiement(total, recu);
    const cadre = element("caisse_rendu_cadre");
    const champ = element("caisse_montant");

    element("caisse_total").textContent = global.format.monnaie(total);
    element("caisse_total_libelle").textContent = enSaisieDePrix()
      ? "Sommation"
      : "Total a payer";

    champ.className = "paiement__montant";
    cadre.className = "paiement__rendu";

    if (etat === "insuffisant") {
      champ.classList.add("paiement__montant--insuffisant");
      cadre.classList.add("paiement__rendu--insuffisant");
      element("caisse_rendu_libelle").textContent = "Manque";
      element("caisse_rendu").textContent = global.format.monnaie(total - recu);
    } else if (etat === "exact") {
      champ.classList.add("paiement__montant--exact");
      cadre.classList.add("paiement__rendu--exact");
      element("caisse_rendu_libelle").textContent = "Compte juste";
      element("caisse_rendu").textContent = "";
    } else if (etat === "rendu") {
      champ.classList.add("paiement__montant--rendu");
      cadre.classList.add("paiement__rendu--rendu");
      element("caisse_rendu_libelle").textContent = "A rendre";
      element("caisse_rendu").textContent = global.format.monnaie(recu - total);
    } else {
      element("caisse_rendu_libelle").textContent = "A rendre";
      element("caisse_rendu").textContent = "";
    }

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

  // Article sans fiche catalogue : seul le prix saisi le definit.
  function ajouterArticleLibre() {
    const centimes = enCentimes(prixSaisi);

    if (!prixSaisi || centimes <= 0) {
      global.notifier.alerte("Saisis d abord un prix superieur a zero.");
      return false;
    }

    panier.push({
      produit: null,
      libelle: "Article " + (panier.length + 1),
      prix_centimes: centimes,
      quantite: 1
    });

    prixSaisi = "";
    afficherPrix();
    afficherPanier();

    return true;
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
    prixSaisi = "";

    if (mode === "manuel") {
      phase = "saisie";
    }

    afficherMontant();
    afficherPrix();
    appliquerIndice();
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

  function ajouterAuTampon(caracteres) {
    if (enSaisieDePrix()) {
      prixSaisi = normaliser(prixSaisi + caracteres);
      afficherPrix();
      afficherTotaux();
      return;
    }

    montantSaisi = normaliser(montantSaisi + caracteres);
    afficherMontant();
    afficherTotaux();
    stabiliserMontant();
  }

  function retirerDuTampon() {
    if (enSaisieDePrix()) {
      prixSaisi = prixSaisi.slice(0, -1);
      afficherPrix();
      afficherTotaux();
      return;
    }

    montantSaisi = montantSaisi.slice(0, -1);
    afficherMontant();
    afficherTotaux();
    stabiliserMontant();
  }

  function effacerTampon() {
    if (enSaisieDePrix()) {
      prixSaisi = "";
      afficherPrix();
      afficherTotaux();
      return;
    }

    montantSaisi = "";
    montantStable = "";
    afficherMontant();
    afficherTotaux();
  }

  function montantExact() {
    montantSaisi = global.format.monnaie(totalCentimes()).replace(/ /g, "");
    montantStable = montantSaisi;
    afficherMontant();
    afficherTotaux();
  }

  // Fin de la sommation manuelle : le total est fige, le curseur passe sur la
  // somme recue du client.
  function totaliser() {
    if (prixSaisi && !ajouterArticleLibre()) {
      return;
    }

    if (!panier.length) {
      global.notifier.alerte("Aucun article : saisis un prix puis touche +.");
      return;
    }

    phase = "paiement";
    element("caisse_bloc_prix").hidden = true;
    element("caisse_ajouter_manuel").hidden = true;
    appliquerIndice();
    afficherTotaux();

    const champ = element("caisse_montant");

    champ.focus();
    champ.select();
  }

  function appliquerIndice() {
    const indice = element("caisse_indice");

    if (mode === "scan") {
      indice.textContent = "Scanne les articles puis tape la somme recue (Entree).";
      return;
    }

    indice.textContent = enSaisieDePrix()
      ? "Tape un prix, touche + pour l ajouter, Entree pour totaliser."
      : "Tape la somme recue du client, Entree pour encaisser.";
  }

  function appliquerMode(nouveau) {
    mode = nouveau === "manuel" ? "manuel" : "scan";
    phase = mode === "manuel" ? "saisie" : "paiement";
    prixSaisi = "";
    montantSaisi = "";
    montantStable = "";

    element("caisse_mode_scan").classList.toggle("bouton_mode--actif", mode === "scan");
    element("caisse_mode_manuel").classList.toggle("bouton_mode--actif", mode === "manuel");
    element("caisse_bloc_prix").hidden = mode !== "manuel";
    element("caisse_ajouter_manuel").hidden = mode !== "manuel";

    afficherPrix();
    afficherMontant();
    appliquerIndice();
    afficherTotaux();

    if (mode === "manuel") {
      element("caisse_prix").focus();
    }
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
          return {
            produit: ligne.produit,
            quantite: ligne.quantite,
            prix: ligne.produit === null
              ? global.format.monnaie(ligne.prix_centimes).replace(/ /g, "")
              : null,
            libelle: ligne.produit === null ? ligne.libelle : null
          };
        }),
        recu: montantSaisi || "0"
      });

      afficherTicket(ticket);

      if (ticket.impression_recu) {
        global.notifier.succes(
          "Ticket no " +
            ticket.numero_ticket +
            " encaisse. A rendre : " +
            global.format.monnaie(ticket.rendu_centimes)
        );
      }

      viderPanier();
      await rafraichirCatalogue();

      if (mode === "manuel") {
        element("caisse_bloc_prix").hidden = false;
        element("caisse_ajouter_manuel").hidden = false;
        element("caisse_prix").focus();
      }

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

  // Impression des recus desactivee : aucune feuille, aucun panneau de ticket,
  // seulement l eclair de confirmation au centre de l ecran. Le rapport Z,
  // lui, s imprime toujours : ce reglage ne touche que la caisse.
  function afficherTicket(ticket) {
    const panneau = element("caisse_ticket_panneau");

    element("caisse_ticket_numero").textContent = String(ticket.numero_ticket);
    element("caisse_ticket_texte").textContent = ticket.texte;

    if (!ticket.impression_recu) {
      panneau.hidden = true;
      global.notifier.eclair(confirmationEclair(ticket));
      return;
    }

    panneau.hidden = !ticket.ticket_actif;

    if (ticket.ticket_actif) {
      global.impression.imprimer(ticket.texte);
    }
  }

  function confirmationEclair(ticket) {
    if (ticket.rendu_centimes > 0) {
      return "Vente encaissee - a rendre " + global.format.monnaie(ticket.rendu_centimes);
    }

    return "Vente encaissee";
  }

  // La touche Entree fait partir l eclair aussitot : le caissier rapide
  // n attend pas la seconde complete avant la vente suivante.
  function eclairAbsorbeEntree(evenement) {
    if (evenement.key !== "Enter" || !global.notifier.eclairAffiche()) {
      return false;
    }

    evenement.preventDefault();
    global.notifier.fermerEclair();

    return true;
  }

  async function surScan(charge) {
    if (!session) {
      global.notifier.alerte("Aucune session ouverte : impossible d encaisser ce scan.");
      return;
    }

    // Les premiers caracteres du code ont pu tomber dans la zone montant :
    // on restitue la derniere valeur stable avant de traiter le code.
    if (!enSaisieDePrix()) {
      montantSaisi = montantStable;
      afficherMontant();
    }

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
    if (evenement.defaultPrevented) {
      return;
    }

    if (eclairAbsorbeEntree(evenement)) {
      return;
    }

    if (!active || !session) {
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

      if (enSaisieDePrix()) {
        totaliser();
      } else {
        encaisser();
      }

      return;
    }

    if (touche === "+" && mode === "manuel") {
      evenement.preventDefault();
      ajouterArticleLibre();
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
      retirerDuTampon();
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
    if (ecart < CADENCE_MACHINE_MS && !enSaisieDePrix()) {
      return;
    }

    evenement.preventDefault();
    ajouterAuTampon(estVirgule ? "," : touche);
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

    // Devise sans subdivision : la virgule disparait du pave, sinon le
    // caissier tape un separateur que la caisse refuse.
    global.format.definirDecimales(nouveaux.decimales);

    const virgule = document.querySelector('[data-touche=","]');

    if (virgule) {
      virgule.hidden = !global.format.avecDecimales();
    }

    montantSaisi = normaliser(montantSaisi);
    prixSaisi = normaliser(prixSaisi);
    afficherMontant();
    afficherPrix();

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

  function brancherChamps() {
    const champPrix = element("caisse_prix");
    const champMontant = element("caisse_montant");

    champPrix.addEventListener("input", function () {
      prixSaisi = normaliser(champPrix.value);
      afficherPrix();
      afficherTotaux();
    });

    champPrix.addEventListener("keydown", function (evenement) {
      if (eclairAbsorbeEntree(evenement)) {
        return;
      }

      if (evenement.key === "+") {
        evenement.preventDefault();
        ajouterArticleLibre();
        return;
      }

      if (evenement.key === "Enter") {
        evenement.preventDefault();
        totaliser();
      }
    });

    champMontant.addEventListener("input", function () {
      montantSaisi = normaliser(champMontant.value);
      montantStable = montantSaisi;
      afficherMontant();
      afficherTotaux();
    });

    champMontant.addEventListener("keydown", function (evenement) {
      if (eclairAbsorbeEntree(evenement)) {
        return;
      }

      if (evenement.key === "Enter") {
        evenement.preventDefault();
        encaisser();
      }
    });
  }

  function initialiser() {
    element("caisse_etat_ticket").addEventListener("click", basculerTicket);
    element("caisse_recherche").addEventListener("input", afficherGrille);
    element("caisse_filtre_rayon").addEventListener("change", afficherGrille);
    element("caisse_encaisser").addEventListener("click", encaisser);
    element("caisse_ajouter_manuel").addEventListener("click", ajouterArticleLibre);
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

    document.querySelectorAll("[data-mode]").forEach(function (noeud) {
      noeud.addEventListener("click", function () {
        appliquerMode(noeud.dataset.mode);
      });
    });

    document.querySelectorAll("[data-touche]").forEach(function (touche) {
      touche.addEventListener("click", function () {
        const valeur = touche.dataset.touche;

        if (valeur === "effacer") {
          effacerTampon();
        } else if (valeur === "retour") {
          retirerDuTampon();
        } else if (valeur === "exact") {
          if (enSaisieDePrix()) {
            totaliser();
          } else {
            montantExact();
          }
        } else {
          ajouterAuTampon(valeur);
        }
      });
    });

    brancherChamps();
    document.addEventListener("keydown", surTouche);
    appliquerMode("scan");
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
