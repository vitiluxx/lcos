# Demande — clavier visuel tactile dans LCOS

**Demandé le** : 2026-11-03
**État** : commencé, interrompu à la demande de l'utilisateur. Rien n'est fini,
rien n'est compilé.

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

Deux travaux distincts, donc :

1. **Un clavier visuel intégré à LCOS**, pas celui de Windows.
2. **Installer de quoi compiler** sur cette machine, puis produire le setup.

## 1. Le clavier visuel — cahier des charges

| Règle | Détail |
|---|---|
| Ouverture | Au moment de saisir dans un champ (clic ou prise de focus) |
| Fermeture | Clic n'importe où ailleurs dans l'interface, **ou** sur la croix `X` du clavier |
| Ne ferme pas | Un appui sur le clavier lui-même, ni sur le champ en cours de saisie |
| Intégré | Composant de LCOS, aucune dépendance, aucun clavier Windows appelé |

Ajouts décidés en cours de route, à conserver :

- La touche **Échap** ferme aussi le clavier (un vrai clavier reste branché sur
  cette caisse).
- Changer de vue ou d'onglet ferme le clavier.
- Deux dispositions : **AZERTY** pour le texte, **pavé chiffres et symboles**
  pour les codes et les montants. Le choix est automatique selon le champ
  (`type="password"`, `inputmode="numeric"`, classe `champ__saisie--chiffre`),
  avec deux onglets `ABC` / `123` pour forcer l'autre.
- **Pas de touches accentuées** : le ticket thermique ne les rend pas.
  `escpos::nettoyer()` remplace tout octet hors ASCII imprimable par `?`. Un
  nom de commerce accentué sortirait donc abîmé sur le rouleau.
- La croix est large : un doigt vise moins bien qu'une souris.

## Pièges repérés avant de coder — à ne pas réapprendre

**La douchette.** `scanner.js` écoute `keydown` sur tout le document et juge
une frappe par sa cadence. Si le clavier visuel émettait de faux événements
clavier, chaque appui compterait comme une touche de code-barres. Deux
mesures :

- le clavier écrit **directement dans la valeur du champ** et n'émet qu'un
  événement `input`, que le scanner ne regarde pas ;
- `scanner.js` doit ignorer les événements dont `isTrusted` est faux — une
  vraie douchette n'en produit jamais. C'est le garde-fou qui reste à poser,
  pour la touche `Entrée` du clavier visuel, qui doit bien être simulée.

**Le focus.** Appuyer sur une touche ne doit pas sortir du champ. Il faut
`preventDefault()` sur le `mousedown` du panneau — pas sur `pointerdown`, qui
supprimerait le clic lui-même sur certains navigateurs.

**La fermeture par clic ailleurs.** Fermer sur `mousedown` change la mise en
page avant le clic : le bouton visé se déplace et le doigt le rate. Il faut
fermer sur le `click`, en phase de **capture** au niveau du document : la cible
du clic est déjà fixée, le bouton reçoit quand même son événement.

**La place à l'écran.** Le défilement n'est pas sur `body` mais sur
`main.contenu` (`overflow-y: auto`, `base.css`). La marge à ouvrir sous le
contenu est donc `body.clavier_ouvert .contenu { padding-bottom: … }`, et le
champ actif est ramené dans la vue par `scrollIntoView`.

## Ce qui est déjà écrit

| Fichier | État |
|---|---|
| `src/styles/composants/clavier.css` | **Écrit.** Panneau fixé en bas, entête avec le nom du champ, onglets `ABC`/`123`, croix, rangées de touches à largeur égale, retour visuel à l'appui, variante écran bas (`max-height: 700px`), marge de défilement du contenu |

## Ce qui reste à faire

1. `src/scripts/composants/clavier.js` — le composant : dispositions, choix
   automatique, insertion au curseur (`selectionStart` / `selectionEnd`),
   retour arrière, effacer, espace, majuscule verrouillable, `Entrée`,
   ouverture sur `focusin`, fermeture (clic capture, croix, Échap).
2. `src/index.html` — le squelette du panneau avant la balise de scripts, la
   feuille `styles/composants/clavier.css` dans le `<head>`, et le script.
3. `src/scripts/peripheriques/scanner.js` — ignorer `isTrusted === false`.
4. `src/scripts/application.js` et `src/scripts/vues/parametrage.js` — fermer
   le clavier au changement de vue et d'onglet.
5. Un attribut d'exclusion (`data-clavier="non"`) pour un champ qui ne doit
   jamais l'ouvrir, si le besoin apparaît à l'usage.

## 2. Installer de quoi compiler, puis produire le setup

Rien n'est installé sur cette caisse : ni `git` utilisable pour la
construction, ni `cargo`, ni `rustup`, ni Node. C'est pourquoi aucune
modification de cette session n'a jamais été compilée.

Il faut donc, **sources montrées et accord donné avant chaque
téléchargement**, conformément à la consigne de départ :

| Ressource | Source officielle | Pourquoi |
|---|---|---|
| Rust (MSVC) | `https://rustup.rs` → `rustup-init.exe` | `cargo`, qui construit `src-tauri` |
| Outils de compilation C++ | `https://visualstudio.microsoft.com/visual-cpp-build-tools/` | Le lieur `link.exe` et le SDK Windows, exigés par la cible `x86_64-pc-windows-msvc` |
| Node.js LTS | `https://nodejs.org` | `@tauri-apps/cli`, qui produit l'installateur |
| WebView2 | déjà présent | Windows 10 19045 le fournit |

Ordre de construction, une fois en place :

```
cargo test --manifest-path src-tauri/Cargo.toml
npx tauri build
```

Le setup sort dans `src-tauri/target/release/bundle/`.

**À vérifier d'abord** : cette machine a-t-elle un accès réseau ? Le serveur
MCP de la session a échoué sur un `ENOTFOUND api.anthropic.com`, ce qui laisse
penser que non, ou qu'un filtrage est en place. Sans réseau, aucun de ces
téléchargements n'aboutira et il faudra apporter les installateurs par clé USB.

## Pour reprendre

Relire ce fichier, puis le point 1 de « ce qui reste à faire ». La feuille de
style donne déjà les noms de classes que le JS doit produire :
`clavier__cadre`, `clavier__entete`, `clavier__cible`, `clavier__onglets`,
`clavier__bascule`, `clavier__fermer`, `clavier__touches`, `clavier__rangee`,
`clavier__touche` et ses variantes `--commande`, `--large`, `--espace`,
`--valider`, `--maj_active`.
