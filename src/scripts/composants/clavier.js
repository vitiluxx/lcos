/*
 * Clavier visuel tactile. La caisse P70L est un ecran tactile sans clavier
 * physique devant le client : sans ce panneau, aucun nom de caissier ni nom de
 * commerce ne peut etre saisi au doigt.
 *
 * Trois choix a ne pas defaire :
 *
 *   - les touches ecrivent DANS la valeur du champ et n emettent qu un
 *     evenement `input`. Elles ne simulent pas de frappe : `scanner.js` juge
 *     une touche par sa cadence et prendrait chaque appui pour un caractere de
 *     code-barres. Seule la touche Entree est simulee, et le scanner ecarte
 *     desormais les evenements dont `isTrusted` est faux ;
 *   - le panneau ne s ouvre que sur un clic, jamais sur une prise de focus :
 *     la vue caisse rend la main au champ code-barres apres chaque vente, et
 *     le clavier remonterait tout seul a chaque ticket ;
 *   - la fermeture par clic ailleurs se fait sur le `click`, pas sur le
 *     `mousedown` : fermer avant le clic replie la page, le bouton vise se
 *     deplace et le doigt le rate.
 *
 * Aucun accent sur les touches : `escpos::nettoyer` remplace tout octet hors
 * ASCII imprimable par un `?`, donc un nom accentue sortirait abime du rouleau.
 */
(function (global) {
  "use strict";

  const element = global.format.element;

  // Types de champ ou une saisie au doigt a un sens. `checkbox`, `radio`,
  // `file` et les boutons n en font pas partie : ils se cliquent.
  const TYPES_SAISIE = ["text", "search", "tel", "password", "email", "url", "number"];

  // `variantes` porte les classes de largeur et de style de la feuille :
  // Retour est large parce que c est la touche la plus visee apres les
  // lettres, et la rater coute une saisie entiere.
  const MAJ = { role: "maj", libelle: "Maj", variantes: ["commande"] };
  const RETOUR = { role: "retour", libelle: "Retour", variantes: ["commande", "large"] };
  const EFFACER = { role: "effacer", libelle: "Effacer", variantes: ["commande"] };
  const ESPACE = { role: "espace", libelle: "Espace", variantes: ["espace"] };
  const VALIDER = { role: "valider", libelle: "Entree", variantes: ["valider"] };

  const DISPOSITIONS = {
    lettres: [
      ["a", "z", "e", "r", "t", "y", "u", "i", "o", "p"],
      ["q", "s", "d", "f", "g", "h", "j", "k", "l", "m"],
      [MAJ, "w", "x", "c", "v", "b", "n", "'", "-", RETOUR],
      [EFFACER, ESPACE, VALIDER]
    ],
    // Chiffres et symboles : codes du gerant, prix, codes-barres, devise.
    chiffres: [
      ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"],
      [",", ".", "-", "+", "*", "/", "=", "%", "(", ")"],
      ["@", "#", "&", "_", ":", ";", "!", "?", "\"", "'"],
      [EFFACER, ESPACE, RETOUR, VALIDER]
    ]
  };

  let cible = null;
  let disposition = "lettres";
  let majuscules = false;
  let valeurALOuverture = "";
  let pret = false;

  function ouvert() {
    return pret && cible !== null;
  }

  function champEligible(noeud) {
    if (!noeud || !noeud.tagName || typeof noeud.closest !== "function") {
      return false;
    }

    const balise = noeud.tagName.toLowerCase();

    if (balise !== "input" && balise !== "textarea") {
      return false;
    }

    if (noeud.disabled || noeud.readOnly) {
      return false;
    }

    // Echappatoire pour un champ qui a deja son propre pave a l ecran : les
    // montants de la caisse, par exemple, que le clavier recouvrirait.
    if (noeud.dataset && noeud.dataset.clavier === "non") {
      return false;
    }

    if (noeud.closest(".clavier")) {
      return false;
    }

    if (balise === "textarea") {
      return true;
    }

    return TYPES_SAISIE.indexOf((noeud.type || "text").toLowerCase()) !== -1;
  }

  // Le champ dit lui-meme ce qu il attend : un code se tape sur le pave des
  // chiffres, un nom sur les lettres.
  function dispositionNaturelle(champ) {
    if ((champ.type || "").toLowerCase() === "password") {
      return "chiffres";
    }

    const mode = (champ.getAttribute("inputmode") || "").toLowerCase();

    if (mode === "numeric" || mode === "decimal" || mode === "tel") {
      return "chiffres";
    }

    if (champ.classList.contains("champ__saisie--chiffre")) {
      return "chiffres";
    }

    return "lettres";
  }

  // Deux champs voisins se ressemblent : l entete nomme celui qui est en
  // cours de saisie, avec son etiquette quand il en a une.
  function nomDuChamp(champ) {
    const etiquette = champ.id
      ? document.querySelector('label[for="' + champ.id + '"]')
      : null;

    if (etiquette && etiquette.textContent.trim()) {
      return etiquette.textContent.trim();
    }

    if (champ.placeholder && champ.placeholder.trim()) {
      return champ.placeholder.trim();
    }

    const aria = champ.getAttribute("aria-label");

    if (aria && aria.trim()) {
      return aria.trim();
    }

    return champ.id || "Saisie";
  }

  // Les champs sans selection (`type="number"` notamment) refusent de lire
  // `selectionStart` : l absence de curseur vaut alors ecriture a la fin.
  function curseur(champ) {
    try {
      if (champ.selectionStart === null || champ.selectionEnd === null) {
        return null;
      }

      return { debut: champ.selectionStart, fin: champ.selectionEnd };
    } catch (incident) {
      return null;
    }
  }

  function poserCurseur(champ, position) {
    try {
      champ.setSelectionRange(position, position);
    } catch (incident) {
      // Champ sans selection : le curseur reste ou le navigateur l a mis.
    }
  }

  function signaler(champ, nom) {
    champ.dispatchEvent(new Event(nom, { bubbles: true }));
  }

  // Ecrire la valeur a la main court-circuite `maxlength`, que le navigateur
  // n applique qu aux vraies frappes : la limite est donc refaite ici, sinon
  // une devise de huit caracteres en accepterait trente au doigt.
  function remplacer(champ, valeur) {
    const limite = champ.maxLength;

    if (limite > 0 && valeur.length > limite) {
      return false;
    }

    champ.value = valeur;

    return true;
  }

  function ecrire(texte) {
    if (!cible) {
      return;
    }

    const champ = cible;
    const valeur = champ.value || "";
    const place = curseur(champ);

    if (!place) {
      remplacer(champ, valeur + texte);
      signaler(champ, "input");

      return;
    }

    const nouvelle = valeur.slice(0, place.debut) + texte + valeur.slice(place.fin);

    if (!remplacer(champ, nouvelle)) {
      return;
    }

    poserCurseur(champ, place.debut + texte.length);
    signaler(champ, "input");
  }

  function reculer() {
    if (!cible) {
      return;
    }

    const champ = cible;
    const valeur = champ.value || "";
    const place = curseur(champ);

    if (!place) {
      champ.value = valeur.slice(0, -1);
      signaler(champ, "input");

      return;
    }

    if (place.debut !== place.fin) {
      champ.value = valeur.slice(0, place.debut) + valeur.slice(place.fin);
      poserCurseur(champ, place.debut);
      signaler(champ, "input");

      return;
    }

    if (place.debut === 0) {
      return;
    }

    champ.value = valeur.slice(0, place.debut - 1) + valeur.slice(place.debut);
    poserCurseur(champ, place.debut - 1);
    signaler(champ, "input");
  }

  function effacer() {
    if (!cible) {
      return;
    }

    cible.value = "";
    poserCurseur(cible, 0);
    signaler(cible, "input");
  }

  // La seule touche simulee. Les vues attendent `Entree` sur le document ou
  // sur le champ pour valider un formulaire ; `scanner.js` ecarte cet
  // evenement parce que son `isTrusted` est faux.
  function valider() {
    if (!cible) {
      return;
    }

    const champ = cible;

    fermer();

    champ.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "Enter",
        code: "Enter",
        bubbles: true,
        cancelable: true
      })
    );
  }

  function basculerMajuscules() {
    majuscules = !majuscules;
    dessiner();
  }

  function appui(descripteur) {
    if (typeof descripteur === "string") {
      ecrire(majuscules ? descripteur.toUpperCase() : descripteur);

      return;
    }

    if (descripteur.role === "maj") {
      basculerMajuscules();
      return;
    }

    if (descripteur.role === "retour") {
      reculer();
      return;
    }

    if (descripteur.role === "effacer") {
      effacer();
      return;
    }

    if (descripteur.role === "espace") {
      ecrire(" ");
      return;
    }

    if (descripteur.role === "valider") {
      valider();
    }
  }

  function creerTouche(descripteur) {
    const commande = typeof descripteur !== "string";
    const libelle = commande
      ? descripteur.libelle
      : majuscules
        ? descripteur.toUpperCase()
        : descripteur;

    const touche = global.format.creer("button", "clavier__touche", libelle);

    touche.type = "button";

    if (commande) {
      descripteur.variantes.forEach(function (variante) {
        touche.classList.add("clavier__touche--" + variante);
      });
    }

    if (commande && descripteur.role === "maj" && majuscules) {
      touche.classList.add("clavier__touche--maj_active");
    }

    touche.addEventListener("click", function () {
      appui(descripteur);
    });

    return touche;
  }

  function dessiner() {
    const touches = element("clavier_touches");

    global.format.vider(touches);

    DISPOSITIONS[disposition].forEach(function (rangee) {
      const ligne = global.format.creer("div", "clavier__rangee");

      rangee.forEach(function (descripteur) {
        ligne.appendChild(creerTouche(descripteur));
      });

      touches.appendChild(ligne);
    });

    element("clavier_lettres").classList.toggle(
      "clavier__bascule--actif",
      disposition === "lettres"
    );
    element("clavier_chiffres").classList.toggle(
      "clavier__bascule--actif",
      disposition === "chiffres"
    );
  }

  function basculerDisposition(nom) {
    if (!DISPOSITIONS[nom]) {
      return;
    }

    disposition = nom;
    dessiner();
  }

  function ouvrir(champ) {
    if (!pret) {
      return;
    }

    const memeChamp = cible === champ;

    cible = champ;

    if (!memeChamp) {
      valeurALOuverture = champ.value || "";
      majuscules = false;
      disposition = dispositionNaturelle(champ);
    }

    element("clavier_cible").textContent = nomDuChamp(champ);
    element("clavier").hidden = false;
    document.body.classList.add("clavier_ouvert");

    dessiner();

    // Le curseur doit etre dans le champ qui recoit les touches : un clic sur
    // l etiquette, lui, laisse le focus ou il etait.
    if (document.activeElement !== champ && typeof champ.focus === "function") {
      champ.focus();
    }

    // Le defilement n est pas sur `body` mais sur `main.contenu` : sans cet
    // appel, un champ en bas de page reste sous les touches.
    if (typeof champ.scrollIntoView === "function") {
      champ.scrollIntoView({ block: "center" });
    }
  }

  // Fermeture : `change` n est emis qu ici, comme le ferait un vrai clavier en
  // quittant le champ, et seulement si la valeur a bouge. L emettre a chaque
  // touche relancerait a chaque caractere les vues qui relisent sur `change`.
  function fermer() {
    if (!pret) {
      return;
    }

    const champ = cible;

    cible = null;
    element("clavier").hidden = true;
    document.body.classList.remove("clavier_ouvert");

    if (champ && (champ.value || "") !== valeurALOuverture) {
      signaler(champ, "change");
    }

    valeurALOuverture = "";
  }

  function surClic(evenement) {
    const panneau = element("clavier");

    if (panneau.contains(evenement.target)) {
      return;
    }

    const vise = champVise(evenement.target);

    if (vise) {
      ouvrir(vise);
      return;
    }

    fermer();
  }

  // Le champ designe par un clic : le champ lui-meme, ou celui que son
  // etiquette commande. Un `<label for>` donne le focus a son champ sans etre
  // ce champ : sans ce detour, cliquer l etiquette fermerait le clavier.
  function champVise(noeud) {
    if (!noeud || typeof noeud.closest !== "function") {
      return null;
    }

    const champ = noeud.closest("input, textarea");

    if (champ) {
      return champEligible(champ) ? champ : null;
    }

    const etiquette = noeud.closest("label");

    if (!etiquette) {
      return null;
    }

    const pour = etiquette.getAttribute("for");
    const commande = pour
      ? document.getElementById(pour)
      : etiquette.querySelector("input, textarea");

    return commande && champEligible(commande) ? commande : null;
  }

  // Deplacement au clavier physique (Tabulation) : le panneau deja ouvert
  // suit le champ, mais ne s ouvre pas de lui-meme.
  function surFocus(evenement) {
    if (!ouvert()) {
      return;
    }

    const champ = evenement.target;

    if (champ === cible || element("clavier").contains(champ)) {
      return;
    }

    if (champEligible(champ)) {
      ouvrir(champ);
      return;
    }

    fermer();
  }

  function surTouchePhysique(evenement) {
    if (evenement.key !== "Escape" || !ouvert()) {
      return;
    }

    // Consomme la touche : sinon le meme Echap refermerait le clavier et
    // defferait en plus le dernier geste de la vue dessous.
    evenement.preventDefault();
    evenement.stopPropagation();
    fermer();
  }

  function initialiser() {
    pret = true;

    // `mousedown` et non `pointerdown` : empecher le `pointerdown` annulerait
    // le clic lui-meme sur certains moteurs. Le champ garde ainsi son focus.
    element("clavier").addEventListener("mousedown", function (evenement) {
      evenement.preventDefault();
    });

    element("clavier_fermer").addEventListener("click", fermer);

    element("clavier_lettres").addEventListener("click", function () {
      basculerDisposition("lettres");
    });
    element("clavier_chiffres").addEventListener("click", function () {
      basculerDisposition("chiffres");
    });

    document.addEventListener("click", surClic, true);
    document.addEventListener("focusin", surFocus, true);
    document.addEventListener("keydown", surTouchePhysique, true);
  }

  global.clavier = {
    initialiser: initialiser,
    fermer: fermer,
    ouvert: ouvert,
    cible: function () {
      return cible;
    }
  };
})(window);
