(function (global) {
  "use strict";

  // Trois voies possibles :
  //   - navigateur : la webview ouvre la fenetre d impression Windows ;
  //   - systeme    : le noyau Rust envoie le texte au spouleur, sans dialogue ;
  //   - escpos     : le noyau Rust envoie les octets ESC/POS a la thermique.
  // La voie retenue vient des reglages ; en cas d echec on retombe toujours
  // sur la fenetre d impression plutot que de perdre le ticket.
  let voie = "navigateur";

  function imprimerParNavigateur(texte) {
    const zone = document.getElementById("zone_impression");

    if (!zone) {
      return;
    }

    zone.textContent = String(texte || "");

    global.focus();
    global.print();
  }

  async function imprimer(texte) {
    const contenu = String(texte || "");

    if (voie === "navigateur") {
      imprimerParNavigateur(contenu);
      return;
    }

    try {
      const imprimante = await global.pont.appeler("imprimer_ticket", { texte: contenu });

      global.notifier.neutre("Ticket envoye a " + imprimante + ".");
    } catch (incident) {
      global.notifier.incident(incident);
      imprimerParNavigateur(contenu);
    }
  }

  function appliquerVoie(mode) {
    voie = mode === "systeme" || mode === "escpos" ? mode : "navigateur";
  }

  global.impression = {
    imprimer: imprimer,
    appliquerVoie: appliquerVoie,
    voie: function () {
      return voie;
    }
  };
})(window);
