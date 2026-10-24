(function (global) {
  "use strict";

  const DECIMALES_DEFAUT = 2;

  // Nombre de decimales affichees, dicte par le parametrage de la devise :
  // deux pour l euro, zero pour le FCFA. La monnaie reste comptee en centimes
  // partout, seule la presentation change.
  let decimales = DECIMALES_DEFAUT;

  function definirDecimales(valeur) {
    decimales = Number(valeur) > 0 ? DECIMALES_DEFAUT : 0;
  }

  function avecDecimales() {
    return decimales > 0;
  }

  // Meme mise en forme que le rendu Rust du ticket : groupes de trois et,
  // quand la devise en a, virgule decimale a deux chiffres.
  function monnaie(somme_centimes) {
    const valeur = Number(somme_centimes) || 0;
    const negatif = valeur < 0;
    const absolu = Math.abs(valeur);
    const unites = Math.floor(absolu / 100);
    const reste = absolu % 100;

    const chiffres = String(unites).split("");
    let groupes = "";

    chiffres.forEach(function (chiffre, rang) {
      if (rang > 0 && (chiffres.length - rang) % 3 === 0) {
        groupes += " ";
      }

      groupes += chiffre;
    });

    const signe = negatif ? "-" : "";

    if (!avecDecimales()) {
      return signe + groupes;
    }

    return signe + groupes + "," + String(reste).padStart(2, "0");
  }

  // Saisie humaine vers centimes. Sans decimale, "500" vaut 500 unites.
  function centimes(saisie) {
    const texte = String(saisie || "").trim();

    if (!texte) {
      return 0;
    }

    const tranches = texte.split(",");
    const unites = entier(tranches[0] || "0", 0);

    if (!avecDecimales()) {
      return unites * 100;
    }

    const partie = (tranches[1] || "").padEnd(2, "0").slice(0, 2);

    return unites * 100 + entier(partie, 0);
  }

  function element(identifiant) {
    return document.getElementById(identifiant);
  }

  function vider(cible) {
    while (cible.firstChild) {
      cible.removeChild(cible.firstChild);
    }
  }

  function creer(balise, classe, texte) {
    const noeud = document.createElement(balise);

    if (classe) {
      noeud.className = classe;
    }

    if (texte !== undefined && texte !== null) {
      noeud.textContent = String(texte);
    }

    return noeud;
  }

  function cellule(texte, classe) {
    return creer("td", classe || null, texte);
  }

  function ligneVide(colonnes, message) {
    const ligne = creer("tr");
    const case_vide = creer("td", "tableau__vide", message);

    case_vide.colSpan = colonnes;
    ligne.appendChild(case_vide);

    return ligne;
  }

  function entier(valeur, defaut) {
    const nombre = parseInt(String(valeur).trim(), 10);

    return Number.isNaN(nombre) ? defaut : nombre;
  }

  // Suppression en deux temps : premier clic arme, second confirme. Aucune
  // dependance a window.confirm, que la webview peut refuser.
  function boutonConfirme(texte, classe, action) {
    const noeud = creer("button", classe, texte);
    let arme = false;
    let minuterie = null;

    function desarmer() {
      arme = false;
      noeud.textContent = texte;
      noeud.classList.remove("bouton--actif");
    }

    noeud.type = "button";
    noeud.addEventListener("click", function () {
      if (!arme) {
        arme = true;
        noeud.textContent = "Confirmer ?";
        noeud.classList.add("bouton--actif");
        minuterie = setTimeout(desarmer, 5000);
        return;
      }

      if (minuterie !== null) {
        clearTimeout(minuterie);
      }

      desarmer();
      action();
    });

    return noeud;
  }

  global.format = {
    monnaie: monnaie,
    centimes: centimes,
    definirDecimales: definirDecimales,
    avecDecimales: avecDecimales,
    element: element,
    vider: vider,
    creer: creer,
    cellule: cellule,
    ligneVide: ligneVide,
    boutonConfirme: boutonConfirme,
    entier: entier
  };
})(window);
