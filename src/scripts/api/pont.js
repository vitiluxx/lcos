(function (global) {
  "use strict";

  function pontDisponible() {
    return Boolean(global.__TAURI__ && global.__TAURI__.core);
  }

  async function appeler(nomCommande, arguments_commande) {
    if (!pontDisponible()) {
      throw new Error("pont natif indisponible : lancer l'application installee");
    }

    return global.__TAURI__.core.invoke(nomCommande, arguments_commande || {});
  }

  global.pont = {
    appeler: appeler,
    disponible: pontDisponible
  };
})(window);
