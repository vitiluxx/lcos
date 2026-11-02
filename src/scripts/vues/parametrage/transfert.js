/*
 * Export et import du contenu de la caisse. Deux portees seulement, jamais de
 * fusion partielle : le parametrage seul, ou le parametrage avec les ventes.
 * Le noyau ecrit et relit un fichier JSON ; ici on ne fait que designer le
 * dossier, montrer ce qui part, et confirmer ce qui revient.
 */
(function (global) {
  "use strict";

  const element = global.format.element;

  let etat = null;

  function octetsLisibles(octets) {
    const valeur = Number(octets) || 0;

    if (valeur < 1024) {
      return valeur + " octets";
    }

    if (valeur < 1024 * 1024) {
      return Math.round(valeur / 1024) + " ko";
    }

    return (valeur / (1024 * 1024)).toFixed(1) + " Mo";
  }

  function libelleContenu(contenu) {
    if (contenu === "complet") {
      return "parametrage et ventes";
    }

    if (contenu === "parametrage") {
      return "parametrage seul";
    }

    return "contenu inconnu";
  }

  function remplirFichiers(fichiers) {
    const liste = element("transfert_fichier");
    const choisi = liste.value;

    global.format.vider(liste);

    if (!fichiers.length) {
      liste.appendChild(new Option("Aucun fichier .json dans ce dossier", ""));
      element("transfert_fichier_aide").textContent =
        "aucun fichier .json dans ce dossier : corrige le dossier plus haut puis Relire le " +
        "dossier, ou tape le chemin complet du fichier ci-dessous";
      return;
    }

    fichiers.forEach(function (fichier) {
      const etiquette =
        fichier.nom +
        "  -  " +
        libelleContenu(fichier.contenu) +
        "  -  " +
        octetsLisibles(fichier.octets) +
        (fichier.utilisable ? "" : "  -  illisible");

      liste.appendChild(new Option(etiquette, fichier.chemin));
    });

    liste.value = choisi;

    if (!liste.value) {
      liste.selectedIndex = 0;
    }

    afficherFichierChoisi();
  }

  // Chemin tape a la main : la cle USB n est pas toujours le dossier
  // d export, et un select vide ne doit pas fermer l import.
  function cheminSaisi() {
    return element("transfert_chemin").value.trim();
  }

  function fichierChoisi() {
    const saisi = cheminSaisi();

    if (saisi) {
      return { chemin: saisi, utilisable: true, contenu: "inconnu", saisi: true };
    }

    const chemin = element("transfert_fichier").value;

    if (!etat || !chemin) {
      return null;
    }

    return (
      etat.fichiers.filter(function (fichier) {
        return fichier.chemin === chemin;
      })[0] || null
    );
  }

  function afficherFichierChoisi() {
    const fichier = fichierChoisi();
    const aide = element("transfert_fichier_aide");

    if (!fichier) {
      aide.textContent = "aucun fichier choisi";
      return;
    }

    if (fichier.saisi) {
      aide.textContent = "chemin tape a la main : " + fichier.chemin;
      return;
    }

    if (!fichier.utilisable) {
      aide.textContent = "ce fichier n est pas une sauvegarde LCOS relisible";
      return;
    }

    aide.textContent =
      libelleContenu(fichier.contenu) +
      ", genere le " +
      (fichier.genere_le || "date inconnue") +
      " par LCOS " +
      (fichier.version_logiciel || "?");
  }

  function appliquerConfirmation() {
    element("transfert_importer").disabled =
      !element("transfert_confirmation").checked || !sousLicence();
  }

  // La remise a neuf n est pas fermee pendant l essai : nettoyer une caisse
  // apres des essais doit rester possible sans licence. Seule la sauvegarde
  // qui l aurait protegee, elle, en demande une.
  function appliquerRemiseANeuf(nouvel) {
    element("transfert_neuf_sauvegarde").textContent = nouvel.derniere_sauvegarde
      ? "Derniere sauvegarde avec les ventes : " + nouvel.derniere_sauvegarde + "."
      : "Aucune sauvegarde avec les ventes n a jamais ete ecrite.";

    element("transfert_neuf_confirmation").checked = false;
    element("transfert_remettre_a_neuf").disabled = true;
  }

  function sousLicence() {
    return !etat || etat.sous_licence;
  }

  // Version d essai : l export et l import sont les seules fonctions fermees.
  // Le reste de l ecran reste lisible, y compris le resume du contenu.
  function appliquerLicence() {
    const essai = !sousLicence();

    element("transfert_essai").hidden = !essai;

    [
      "transfert_exporter_parametrage",
      "transfert_exporter_complet",
      "transfert_fichier",
      "transfert_chemin",
      "transfert_avec_donnees",
      "transfert_confirmation"
    ].forEach(function (identifiant) {
      element(identifiant).disabled = essai;
    });
  }

  function appliquer(nouvel) {
    etat = nouvel;

    element("transfert_produits").textContent = String(nouvel.resume.produits);
    element("transfert_rayons").textContent = String(nouvel.resume.rayons);
    element("transfert_comptes").textContent = String(nouvel.resume.comptes);
    element("transfert_parametres").textContent = String(nouvel.resume.parametres);
    element("transfert_ventes").textContent = String(nouvel.resume.ventes);
    element("transfert_rapports").textContent = String(nouvel.resume.rapports_z);

    const dossier = element("transfert_dossier");

    if (!dossier.value.trim()) {
      dossier.value = nouvel.dossier;
    }

    element("transfert_dossier_aide").textContent =
      "par defaut : " + nouvel.dossier_defaut;

    remplirFichiers(nouvel.fichiers);
    appliquerLicence();
    appliquerConfirmation();
    appliquerRemiseANeuf(nouvel);
  }

  async function rafraichir() {
    if (!global.acces.actif()) {
      return;
    }

    try {
      appliquer(
        await global.pont.appeler("recuperer_etat_transfert", {
          jeton: global.acces.exiger(),
          dossier: element("transfert_dossier").value
        })
      );
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  async function exporter(avecDonnees) {
    try {
      const resultat = await global.pont.appeler("exporter_sauvegarde", {
        jeton: global.acces.exiger(),
        dossier: element("transfert_dossier").value,
        avecDonnees: avecDonnees
      });

      element("transfert_dernier_export").textContent =
        "ecrit : " + resultat.chemin + " (" + octetsLisibles(resultat.octets) + ")";

      global.notifier.succes(
        "Export " +
          libelleContenu(resultat.contenu) +
          " : " +
          resultat.resume.produits +
          " produits, " +
          resultat.resume.comptes +
          " comptes" +
          (avecDonnees ? ", " + resultat.resume.ventes + " ventes" : "") +
          "."
      );

      await rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  // Apres un import, le code du gerant est celui du fichier : le menu se
  // reverrouille, sinon le gerant resterait entre avec un jeton obtenu par un
  // code qui n existe plus.
  async function importer() {
    const fichier = fichierChoisi();

    if (!fichier) {
      global.notifier.alerte("Choisis un fichier dans le dossier.");
      return;
    }

    if (!fichier.utilisable) {
      global.notifier.alerte("Ce fichier n est pas une sauvegarde LCOS relisible.");
      return;
    }

    const avecDonnees = element("transfert_avec_donnees").checked;

    try {
      const resultat = await global.pont.appeler("importer_sauvegarde", {
        jeton: global.acces.exiger(),
        chemin: fichier.chemin,
        avecDonnees: avecDonnees
      });

      element("transfert_confirmation").checked = false;
      appliquerConfirmation();

      global.notifier.succes(
        "Import termine : " +
          resultat.resume.produits +
          " produits, " +
          resultat.resume.rayons +
          " rayons, " +
          resultat.resume.comptes +
          " comptes" +
          (avecDonnees ? ", " + resultat.resume.ventes + " ventes" : "") +
          "."
      );
      global.notifier.neutre(
        "Parametrage reverrouille : utilise desormais le code du gerant du fichier."
      );

      await global.vueParametrage.verrouiller();
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      global.notifier.incident(incident);
    }
  }

  // Remise a neuf : tout l historique et tous les compteurs partent ensemble,
  // la programmation reste. Le noyau refuse tant qu une session de caisse est
  // ouverte, l ecran n a donc pas a s en charger.
  async function remettreANeuf() {
    const resultat = element("transfert_neuf_resultat");

    try {
      const bilan = await global.pont.appeler("remettre_la_caisse_a_neuf", {
        jeton: global.acces.exiger()
      });

      resultat.textContent =
        "Efface : " +
        bilan.ventes +
        " vente(s), " +
        bilan.sessions +
        " session(s), " +
        bilan.rapports_z +
        " rapport(s) Z. Grands totaux, compteur de remises a zero et numero de ticket" +
        " repartis de zero. Programmation conservee.";
      resultat.hidden = false;

      global.notifier.succes("Caisse remise a neuf. La programmation n a pas bouge.");

      await rafraichir();
      global.vueRapports.rafraichir();
      global.application.rafraichirEtat();
      global.vueCaisse.rafraichir();
    } catch (incident) {
      resultat.hidden = true;
      global.notifier.incident(incident);
    }
  }

  function initialiser() {
    element("transfert_relire").addEventListener("click", rafraichir);
    element("transfert_relire_import").addEventListener("click", rafraichir);
    element("transfert_chemin").addEventListener("input", afficherFichierChoisi);
    element("transfert_dossier").addEventListener("change", rafraichir);
    element("transfert_fichier").addEventListener("change", afficherFichierChoisi);
    element("transfert_confirmation").addEventListener("change", appliquerConfirmation);
    element("transfert_importer").addEventListener("click", importer);

    element("transfert_remettre_a_neuf").addEventListener("click", remettreANeuf);

    element("transfert_neuf_confirmation").addEventListener("change", function (evenement) {
      element("transfert_remettre_a_neuf").disabled = !evenement.target.checked;
    });

    element("transfert_exporter_parametrage").addEventListener("click", function () {
      exporter(false);
    });
    element("transfert_exporter_complet").addEventListener("click", function () {
      exporter(true);
    });
  }

  global.reglageTransfert = {
    initialiser: initialiser,
    rafraichir: rafraichir
  };
})(window);
