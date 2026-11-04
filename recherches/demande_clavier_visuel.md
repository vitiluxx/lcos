# Demande — clavier visuel tactile dans LCOS

**Demandé le** : 2026-11-03
**État** : **livré et mesuré** dans le setup `LCOS_1.9.9_x64-setup.exe`
(2026-11-04). Plus rien à reprendre.

## La demande, telle qu'elle a été formulée

> Vu que j'utilise aussi beaucoup le mode manuel avec l'écran tactile,
> actuellement lorsque je clique sur un champ pour saisir (exemple le nom d'un
> caissier ou autre) une information il n'y a pas de clavier visuel à l'écran.
> Si tu peux intégrer un clavier visuel directement à LCOS qui s'ouvre lorsque
> nous souhaitons faire une saisie sur un champ et ce clavier doit se refermer
> si nous cliquons sur tout autre endroit de l'interface que sur le clavier
> lui-même ou directement sur sa croix « X » de fermeture. Si tu finis, tu
> installes les ressources nécessaires pour lancer la compilation et avoir le
> setup du logiciel.

## 1. Le clavier visuel — ce qui a été livré

Règles tenues :

- **Ouverture** : au clic sur un champ de saisie, ou sur l'étiquette
  `<label for>` qui le commande.
- **Fermeture** : clic n'importe où ailleurs, croix `X`, touche `Échap`,
  changement de vue, changement de sous-onglet du paramétrage.
- **Ne ferme pas** : un appui sur le clavier lui-même, ni sur le champ en cours
  de saisie.
- **Intégré** : composant de LCOS, aucune dépendance, aucun clavier Windows
  appelé.
- Deux dispositions : **AZERTY** pour le texte, **chiffres et symboles** pour
  les codes et les montants. Le choix est automatique (`type="password"`,
  `inputmode="numeric"` ou `decimal`, classe `champ__saisie--chiffre`), avec
  deux onglets `ABC` / `123` pour forcer l'autre.
- `Maj` est **verrouillable** : elle reste active jusqu'au second appui.
- **Aucune touche accentuée** : `escpos::nettoyer()` remplace tout octet hors
  ASCII imprimable par `?`, un nom de commerce accentué sortirait abîmé du
  rouleau.
- La croix est large : un doigt vise moins bien qu'une souris.

Fichiers :

- `src/styles/composants/clavier.css` — panneau fixé en bas, entête nommant le
  champ, onglets, croix, rangées à largeur égale, retour visuel à l'appui,
  variante écran bas, marge de défilement du contenu.
- `src/scripts/composants/clavier.js` — le composant complet.
- `src/index.html` — feuille liée, squelette du panneau avant les
  notifications, script après les autres composants, et `data-clavier="non"`
  sur `caisse_prix`, `caisse_quantite` et `caisse_montant`.
- `src/scripts/peripheriques/scanner.js` — `if (!evenement.isTrusted) return;`
- `src/scripts/application.js` — `global.clavier.initialiser()` au démarrage,
  `fermer()` à chaque bascule de vue.
- `src/scripts/vues/parametrage.js` — `fermer()` à chaque bascule de
  sous-onglet.

## Pièges repérés — à ne pas réapprendre

**La douchette.** `scanner.js` écoute `keydown` sur tout le document et juge
une frappe par sa cadence. Deux mesures, toutes deux en place :

- les touches écrivent **directement dans la valeur du champ** et n'émettent
  qu'un événement `input`, que le scanner ne regarde pas ;
- `scanner.js` ignore les événements dont `isTrusted` est faux — une vraie
  douchette n'en produit jamais. C'est ce qui permet à la touche `Entrée`, la
  seule simulée, de valider un champ sans clôturer un faux code-barres.

**Le focus.** `preventDefault()` sur le `mousedown` du panneau, et non sur
`pointerdown`, qui supprimerait le clic lui-même sur certains moteurs.

**La fermeture par clic ailleurs.** Sur le `click` en phase de **capture** au
niveau du document : fermer sur `mousedown` replie la page avant le clic, le
bouton visé se déplace et le doigt le rate.

**La place à l'écran.** Le défilement n'est pas sur `body` mais sur
`main.contenu` : la marge est `body.clavier_ouvert .contenu { padding-bottom }`
et le champ actif est ramené dans la vue par `scrollIntoView`.

**`maxlength` ne s'applique pas.** Le navigateur ne la fait respecter qu'aux
vraies frappes : écrire `value` à la main la contourne. La limite est donc
refaite dans le composant, sinon un champ de huit caractères en accepterait
trente au doigt.

**`selectionStart` jette sur certains champs.** `type="number"` refuse de la
lire : l'absence de curseur vaut alors écriture à la fin.

**Le `change`.** Il n'est émis qu'à la fermeture, et seulement si la valeur a
bougé, comme le ferait un vrai clavier en quittant le champ. L'émettre à chaque
touche relancerait à chaque caractère les vues qui relisent sur `change`.

**Les étiquettes.** Un `<label for>` donne le focus à son champ sans être ce
champ : sans le détour par `for`, cliquer l'étiquette fermait le clavier.

## Recette passée sur l'exécutable 1.9.9

Mesurée par le protocole de débogage de la webview, pas relue :

- clic sur « Nom du commerce » → ouvert, entête `"Nom du commerce"`, **ABC**,
  marge de défilement ouverte ;
- `abc` → `"abc"` ; `Maj` puis `AB` → `"AB"` ; `Maj` relâchée puis `c` →
  `"ABc"` ;
- curseur placé en position 1 puis `z` → `"AzBc"`, curseur ressorti à 2 ;
- `Retour`, `Espace`, `Effacer` → `"ab"`, `"ab f"`, `""` ;
- onglet `123` puis `12,5` → bascule faite, champ à `"12,5"` ;
- champ `maxlength="8"`, dix touches tapées → **huit** caractères ;
- clic ailleurs, croix, `Échap`, changement d'onglet, changement de vue →
  fermé dans les cinq cas ;
- clic sur l'étiquette → ouvert sur son champ ;
- clic sur « Code secret » (`type="password"`) → ouvert d'emblée en **123** ;
- `Entrée` → reçue une fois par le champ, clavier fermé, **0 scan et 0 rejet**
  côté douchette ;
- clic sur « Somme reçue du client » → reste fermé (exclu) ; clic sur
  « Chercher un article » → ouvert en ABC ;
- `change` émis **une** fois après une saisie, et **pas** après une ouverture
  sans frappe.

## Un choix assumé, à trancher à l'usage

Le panneau **ne s'ouvre jamais sur une prise de focus**, seulement au clic. La
vue caisse rend la main au champ code-barres après chaque vente : sur
`focusin`, le clavier remonterait à chaque ticket.

Conséquence : sur l'écran du verrou, le code du gérant est déjà focalisé mais
il faut **toucher le champ une fois** pour faire venir le clavier. Si cette
exception devient gênante, l'ouvrir pour ce seul champ se fait dans
`pointerSaisie()` (`src/scripts/vues/parametrage.js`), pas en rétablissant
l'ouverture sur `focusin`.

## 2. De quoi compiler

Sans objet sur le poste de développement : `cargo 1.99.1` et `node v24.20.0` y
sont installés, et c'est là qu'ont été produits tous les setups.

Sur la caisse P70L, rien n'est installé et rien n'a besoin de l'être : elle
reçoit le setup, pas les sources. Si la compilation devait un jour s'y faire,
il faudrait Rust MSVC (`https://rustup.rs`), les outils de compilation C++
(`https://visualstudio.microsoft.com/visual-cpp-build-tools/`) et Node LTS
(`https://nodejs.org`) — WebView2 y est déjà fourni par Windows 10.

Ordre de construction, sur le poste de développement :

```
cargo test --lib --manifest-path src-tauri/Cargo.toml
cargo tauri build
```

Le setup sort dans `src-tauri/target/release/bundle/nsis/`.
