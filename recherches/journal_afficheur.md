# Journal de l'afficheur client — caisse P70L

## Conclusion — résolu le 2026-10-31

| Réglage | Valeur |
|---|---|
| Voie | `serie` |
| Port | **COM2** |
| Débit | **2400** bauds, 8N1 |
| Jeu de commandes | **cd5220** |
| Colonnes | **8** |

**L'afficheur est numérique**, pas alphanumérique : huit positions, chiffres
et point décimal seulement. Il ignore silencieusement les lettres, la
virgule et les espaces. Il **cale à droite tout seul** et ne garde que les
huit derniers caractères reçus — les deux mêmes conventions que `caler()`
dans `domaine/afficheur.rs`, qui tronque par la gauche pour garder les
centimes.

Le `0.00` du départ n'était pas un message d'initialisation : c'est **un
montant à zéro**.

**Correction apportée au code** : `caler()` convertit maintenant la virgule
décimale en point et retire les séparateurs de milliers. Sans elle,
`1 500,00` s'affichait `150000` — cent cinquante mille au lieu de mille cinq
cents, sans qu'aucun envoi n'échoue pour le signaler.

**`ESC @` (`1B 40`) à 2400 sur COM2 réveille l'afficheur** quand l'écran est
noir. Plus besoin de débrancher le cordon.

## Modifications du code source — à lire avant de compiler

Trois fichiers touchés, le 2026-11-01.

| Fichier | Ce qui change | Pourquoi |
|---|---|---|
| `src-tauri/src/domaine/afficheur.rs` | `caler()` convertit la virgule décimale en point et retire les séparateurs de milliers. 7 assertions mises à jour, 1 test ajouté (`met_la_somme_a_la_forme_que_l_ecran_sait_rendre`) | Sans ça, `1 500,00` s'affichait **`150000`** devant le client |
| `src/scripts/vues/afficheur.js` | Ajout de `pousser()`, `liberer()`, `expirerRendu()` et `poserRendu()`. Trois variables de verrou (`renduJusqu`, `relacheRendu`, `sommeDifferee`) et une constante `DUREE_RENDU_MS`. `poserRendu` exposé dans `global.vueAfficheur` | Montrer au client ce qu'on lui rend, et empêcher que la remise à zéro du panier l'efface aussitôt |
| `src/scripts/vues/caisse.js` | Appel de `poserRendu(ticket.rendu_centimes)` dans `encaisser()`, **avant** `viderPanier()` | Envoyer le rendu pendant que le montant est encore connu |

**Rien d'autre n'a été touché** : aucune migration de base, aucun réglage
nouveau, aucune dépendance ajoutée, aucune commande Tauri créée.
`afficher_somme_client` et son paramètre `decimales` existaient déjà.

**Déjà en place, non modifié** : le total poussé en temps réel à chaque
article, et l'option d'activer ou désactiver les décimales.

**État : non compilé.** Ni `cargo` ni Node ne sont installés sur la caisse,
donc ni `cargo test` ni un contrôle de syntaxe JS n'ont été lancés.
L'algorithme de `caler()` a été rejoué à la main en PowerShell contre ses
propres assertions (10 cas sur 10) ; le verrou du rendu n'est validé que par
relecture.

**À essayer en premier après compilation** : encaisser une vente avec un
billet plus gros que le total. Le rendu doit s'afficher et **tenir environ 8
secondes** avant que l'écran revienne à zéro. Si ce délai ne convient pas,
c'est `DUREE_RENDU_MS` en haut de `afficheur.js`.

### Ajustements ultérieurs, et pourquoi ils comptent

| Ajustement | Fichier | Ce qu'il évite |
|---|---|---|
| `assurer_vitesse()` appelé dans `envoyer()`, avec mémoire par couple port/débit (`VITESSE_POSEE`) | `peripheriques.rs` | La consigne `mode` ne survit pas à un redémarrage du poste. Sans cet appel, le port repartait à la vitesse par défaut de Windows et l'afficheur ne comprenait plus rien, puisque personne n'ouvre le paramétrage après chaque démarrage. Un seul processus `mode` par lancement, pas un par frappe |
| `oublier_vitesse()` si l'écriture échoue | `peripheriques.rs` | La vitesse est reposée au retour de l'appareil, au lieu d'être crue acquise |
| `INTERVALLE_MINIMAL_MS = 320` | `afficheur.js` | Chaque envoi ouvre et referme le port. Cent ouvertures en seize secondes ont éteint cet afficheur (essais 4 et 13) : le plancher plafonne à trois envois par seconde |
| `poserSomme(montantSaisi === "" ? total : recu)` | `caisse.js` | Le client voit le total, puis ce qu'on tape de son billet. C'est ce choix qui multiplie les envois, d'où le plancher ci-dessus |

### Écritures inutiles dans la base, corrigées le 2026-11-01

Trouvées en cherchant pourquoi l'onglet « Commerce et ticket » fige une à
deux secondes à l'ouverture. Deux chemins écrivaient dans SQLite sans raison,
et l'un des deux est celui de l'afficheur.

| Où | Ce qui se passait | Correction |
|---|---|---|
| `afficher_somme_client` (`peripheriques.rs`) | `parametres::horodater()` à **chaque** envoi. Or la somme part à chaque article scanné et à chaque chiffre tapé : plusieurs écritures par seconde de vente, pour une date que le gérant lit à la minute près | Horodatage au plus une fois par minute, via `REPOS_HORODATAGE` et un instant retenu en mémoire |
| `jour_de_reference` (`licence.rs`) | Réécrivait la même date à chaque `recuperer_etat_application`, relancé toutes les 20 s par `setInterval` : plus de 4 000 écritures par jour | Écriture seulement quand la date change |
| `connexion.rs` | Rien ne repliait le journal WAL | `PRAGMA wal_checkpoint(TRUNCATE)` au lancement |

**Pourquoi ça gèle.** En mode WAL, SQLite recopie le journal dans la base dès
qu'il dépasse mille pages, soit 4 Mo. Sur cette caisse le journal mesurait
**4 124 152 octets** et le fichier `lcos.db` n'avait pas bougé depuis le
**27 octobre** : le journal était collé à son seuil, la recopie était retentée
à chaque écriture sans jamais aboutir, et chaque tentative fige la fenêtre le
temps qu'elle dure.

### Le piège du protocole par défaut

**Il faut choisir CD5220 et enregistrer. Laisser le réglage vide éteint
l'écran.**

`charger_afficheur()` retombe sur `PROTOCOLE_TEXTE` quand rien n'est
enregistré (`peripheriques.rs`), et `trame()` en protocole texte commence par
`0C`. Or sur cet afficheur, `0C` seul à 2400 **éteint l'écran** (essai 20), et
le texte brut ne s'y affiche pas (essai 21). Activer l'afficheur sans choisir
le protocole ne donne donc pas un affichage absent : il donne un écran noir,
qu'il faut ensuite réveiller par `ESC @`.

En CD5220, LCOS n'envoie **jamais** `0C` : l'écriture est `ESC Q A`, qui
efface la ligne avant d'écrire, et l'effacement entre deux clients est
`ESC Q A` suivi d'espaces. Le risque disparaît avec le bon protocole.

## Ce qui a été implémenté dans LCOS

### 1. Le total suit la vente en temps réel — déjà en place

`afficherTotaux()` dans `src/scripts/vues/caisse.js` appelle
`global.vueAfficheur.poserSomme(total)`. Elle est déclenchée à chaque rendu
du panier, donc **à chaque article scanné, ajouté à la main ou retiré**,
ainsi qu'à chaque frappe dans le champ du montant reçu.

Deux garde-fous étaient déjà là et sont conservés :

- **une rafale de frappes ne produit qu'un envoi.** Un port série lent
  n'encaisse pas deux trames rapprochées sans les mélanger : seule la
  dernière somme part, après 120 ms de calme (`DELAI_ENVOI_MS`) ;
- **une panne de l'afficheur n'arrête jamais une vente.** Le premier échec
  lève un badge et une alerte, puis l'afficheur se tait définitivement
  (`enPanne`) pour ne pas noyer le caissier à chaque frappe.

### 2. Le rendu de monnaie — ajouté

Il manquait. `caisse.js` l'écartait explicitement : *« L'écran du client suit
le total, jamais la somme reçue »*. Le montant à rendre ne s'affichait que
devant le caissier, dans `caisse_rendu`.

**La difficulté n'était pas d'envoyer le rendu, mais de le faire tenir.**
Après l'encaissement, `viderPanier()` remet le total à zéro, ce qui rappelle
`poserSomme(0)` dans la même foulée : le rendu aurait clignoté puis disparu
avant que le client ait pu le lire.

D'où un verrou temporaire dans `src/scripts/vues/afficheur.js` :

- `poserRendu(centimes)` affiche le rendu et **retient l'écran 8 secondes**
  (`DUREE_RENDU_MS`) ;
- pendant ce temps, une somme **nulle** est mise de côté (`sommeDifferee`)
  et partira à l'expiration — c'est le panier vide qui attend son tour ;
- une somme **non nulle** lève le verrou aussitôt : c'est la vente suivante
  qui commence, et le client d'après ne doit pas lire la monnaie du
  précédent ;
- un rendu nul ou négatif n'affiche rien : compte juste, le total encaissé
  reste devant le client.

L'envoi est passé par une fonction unique, `pousser()`, que `poserSomme` et
`poserRendu` partagent.

### 3. L'option décimales — déjà en place

Elle existait déjà, et couvre exactement le cas des monnaies qui ne se
subdivisent pas :

- **dans l'interface** : Paramétrage → Commerce → « Valeurs après la
  virgule », `commerce_decimales` dans `index.html`, au choix « Activées : 2
  décimales (1 500,00) » ou « Désactivées : entiers pleins (1 500) ». La
  valeur est proposée automatiquement d'après la devise saisie ;
- **côté affichage** : `afficheur.js` transmet `decimales: 2` ou `0` à
  chaque envoi, d'après `global.format.avecDecimales()` ;
- **côté calcul** : `monnaie::formater_avec(centimes, decimales)` rend
  `1 234,50` à deux décimales et `1 234` à zéro. Les montants restent
  stockés et calculés **en centimes** dans tous les cas : seule la
  présentation change, jamais l'arithmétique.

### 4. La virgule décimale — corrigé

Voir l'essai 26. `caler()` dans `src-tauri/src/domaine/afficheur.rs`
convertit maintenant la virgule en point et retire les séparateurs de
milliers, espaces insécables et fines insécables comprises. Sept assertions
existantes ont été mises à jour et un test a été ajouté,
`met_la_somme_a_la_forme_que_l_ecran_sait_rendre`, qui fixe le comportement
avec la mesure réelle en commentaire.

C'est le point de passage unique des six protocoles, et il n'est utilisé que
dans ce fichier : la correction vaut donc pour la voie série comme pour la
voie imprimante.

## Ce qui est vérifié, et ce qui ne l'est pas

**Vérifié sur le matériel** : port, débit, jeu de commandes, largeur, calage
à droite natif, rendu du point décimal, rejet de la virgule et des lettres.
Trente essais, consignés ci-dessous.

**Vérifié par raisonnement seulement** : l'algorithme de `caler()` et ses
comptes d'espaces, rejoués en PowerShell contre les assertions des tests
Rust — dix cas sur dix concordent. Ce contrôle a d'ailleurs attrapé une
erreur d'un espace dans ce qui avait été écrit.

**Non vérifié** : la compilation. **Rust n'est pas installé sur la caisse**
(ni `cargo`, ni `.rustup`, ni dossier `target`), et **Node non plus**, donc
ni `cargo test` ni un contrôle de syntaxe JS n'ont pu être lancés. Les deux
fichiers JS modifiés ont été relus et sont en ASCII pur, comme le reste du
projet. **À compiler sur une autre machine avant de s'y fier.**

## Pièges à ne pas réveiller

- **Ne jamais installer d'imprimante du spouleur sur COM2.** Elle réserve le
  port, `SerialPort.Open` échoue alors en `ArgumentException`, et retirer
  l'imprimante ne répare pas : il faut redémarrer le poste. C'est ce qui
  était arrivé à COM1.
- **Ne jamais envoyer `04 01 58 17`**, la remise aux réglages d'usine : elle
  forcerait l'afficheur à 9600 bauds alors qu'il est à 2400, et il faudrait
  pouvoir lui parler pour le rétablir.
- **Ne jamais marteler le port.** Cent ouvertures en seize secondes
  l'éteignent. Une seule ouverture par essai.

---

Traçabilité des essais sur le petit écran tourné vers le client.
Un test, une ligne. Chaque ligne note **l'état avant**, **l'envoi exact** et
**l'état après**, parce que la plupart de nos fausses pistes sont venues de
conclusions tirées sur un écran dont on ne connaissait plus l'état.

## Protocole d'essai

1. L'écran doit afficher `0.00` avant de commencer. C'est son état
   d'initialisation, et le seul témoin fiable.
2. **Un seul test à la fois.** Après chaque envoi, arrêt complet et question :
   *l'écran est-il toujours allumé, ou parti ?*
3. La réponse est consignée ici avant de lancer le test suivant.
4. Si l'écran est parti, on s'arrête : plus rien n'est interprétable tant
   qu'il n'est pas revenu.
5. **Jamais de rafale.** C'est une rafale qui a brouillé les essais 4 à 17.

## Réveil

**`ESC @` (`1B 40`) à 2400 bauds sur COM2 ramène `0.00`.** Établi à l'essai
22. C'est le remède, et il rend `0.00` réutilisable comme témoin autant de
fois qu'on veut :

```
outils\sonder_afficheur_com2.ps1 -Debits 2400 -Trame init
```

Avant de le connaître, le seul moyen était de **débrancher le cordon
secteur** une vingtaine de secondes. Un redémarrage de Windows ne suffit
pas — il ne coupe pas l'alimentation de l'afficheur. À garder en dernier
recours.

## Ce qui est établi

- **L'afficheur est sur COM2.** Prouvé causalement : un martèlement de COM2
  l'éteint, deux fois sur deux. Le même traitement sur COM1 pendant 78 s ne
  provoque rien.
- **COM1 ne porte rien.**
- **LPT1 ne porte pas l'afficheur.** La carte de l'imprimante comprend bien
  la bascule `ESC = 2` — un texte nu envoyé par `copy /b` fait avancer le
  papier, le même précédé de la bascule ne fait rien avancer — mais elle
  dévie vers une prise d'afficheur inutilisée. Les six jeux de commandes y
  sont passés sans effet, écran allumé.
- **Deux preuves négatives ne valent rien ici** : la sonde système
  `04 01 54 17` reste muette car la ligne TX de l'afficheur n'est pas câblée,
  et les lignes de contrôle restent basses car ce câble ne ponte ni
  DTR→DSR ni RTS→CTS.
- **9600 bauds ne produit aucun effet** sur COM2 (essai 14, envoi unique et
  doux). Ce n'est probablement pas le débit.

## Hypothèse principale à vérifier

**`0C` n'efface pas l'écran : il l'éteint.**

Tous nos envois commencent par cet octet. Sur ce modèle, il pourrait
commander une extinction ou une mise en veille plutôt qu'un effacement. Si
c'est le cas :

- au **bon** débit, `0C` éteint l'écran — ce qu'on observe aux essais 4, 13
  et 15 ;
- aux **mauvais** débits, rien ne se passe — ce qu'on observe à l'essai 14.

**Conséquence** : le débit qui éteint l'écran est le **bon** débit. Et pour
afficher, il faut envoyer du texte **sans** `0C`.

Deux hypothèses concurrentes, à ne pas écarter :

- l'extinction vient des **ouvertures/fermetures répétées du port**, DTR et
  RTS retombant à chaque fermeture (le martèlement en fait cent en seize
  secondes) ;
- l'extinction vient d'un **octet déformé** reçu à un débit faux, qui tombe
  par hasard sur une commande de veille ou de luminosité.

Les essais 18 et suivants les séparent : un envoi unique ne peut pas être
imputé aux ouvertures répétées.

## Essais déjà faits

Session du 2026-10-31. Les essais 5 à 11 ont été menés sur un écran déjà
éteint : **ils ne prouvent rien** et ne sont là que pour éviter de les
refaire.

| # | Voie | Envoi | Avant | Après | Ce qu'on en tire |
|---|---|---|---|---|---|
| 1 | COM1 9600 | `1B 40` `0C` texte `0D` | `0.00` | `0.00` | aucun effet |
| 2 | COM1, 7 débits | idem | `0.00` | `0.00` | aucun effet |
| 3 | COM1, martèlement 78 s, 8 débits | `0C` + pleins / vides | `0.00` | `0.00` | COM1 n'atteint rien |
| 4 | COM2, martèlement 16 s, 5 débits | idem | `0.00` | **éteint** | **COM2 atteint l'afficheur** |
| 5 | COM2 9600 | `1B 40` `0C` texte `0D` | éteint | éteint | nul |
| 6 | COM2, 4 débits | idem | éteint | éteint | nul |
| 7 | COM2, 5 débits, sans ESC | `0C` + 2 lignes | éteint | éteint | nul |
| 8 | LPT1 | `ESC = 2` `0C` texte | éteint | éteint | nul |
| 9 | LPT1, texte nu | texte seul | éteint | éteint, papier avance | `copy /b` atteint la carte |
| 10 | LPT1, 6 protocoles | — | éteint | éteint | nul |
| 11 | LPT1, réveil | `ESC @`, `ESC * 4`, pleins | éteint | éteint | nul |
| — | **coupure du cordon** | — | éteint | **`0.00`** | seul réveil connu |
| 12 | LPT1, 6 protocoles | — | `0.00` | `0.00` | **LPT1 n'atteint pas l'afficheur** |
| 13 | COM2, martèlement 16 s | comme 4 | `0.00` | **éteint** | **reproduit** |
| — | **coupure de courant** | — | éteint | **`0.00`** | réveil |
| 14 | COM2 9600, envoi unique | `0C` + `LCOS 9600` | `0.00` | `0.00` | 9600 sans effet |
| 15 | COM2, 5 débits doux | `0C` + `LCOS <n>` | `0.00` | **vide ou éteint** | un débit porte, lequel ? |
| 16 | COM2 19200 puis 4800 | idem | vide/éteint | inchangé | nul, témoin perdu |
| 17 | COM2 2400, 38400, 1200 | idem | vide/éteint | inchangé | nul, témoin perdu |

L'essai 15 est le pivot : un des cinq débits a agi, mais la rafale a effacé
la réponse. Les essais 16 et 17 ont été menés sans témoin, écran déjà
vide ou éteint — ils ne tranchent rien.

## Plan des prochains essais

Un par un, écran à `0.00` au départ, arrêt et question après chacun.

| # | Envoi | Ce que la réponse apprendra |
|---|---|---|
| 18 | COM2 19200, `0C` **seul** | Si l'écran part, 19200 est le débit **et** `0C` éteint. Un envoi unique disculpe les ouvertures répétées. |
| 19 | COM2 4800, `0C` seul | idem pour 4800 |
| 20 | COM2 2400, `0C` seul | idem pour 2400 |
| 21 | COM2 38400, `0C` seul | idem pour 38400 |
| 22 | COM2 1200, `0C` seul | idem pour 1200 |
| 23 | Au débit trouvé : texte **sans** `0C` | Le test décisif de l'hypothèse. Si le texte s'affiche, on a port, débit et jeu de commandes. |

Si aucun des essais 18 à 22 n'éteint l'écran, l'extinction vient des
ouvertures répétées et non d'un octet : on reprendra alors par le texte nu,
sans jamais envoyer `0C`.

## Essais un par un — en cours

Protocole respecté : un envoi, arrêt, question, consignation.

| # | Débit | Octets | Avant | Après | Ce qu'on en tire |
|---|---|---|---|---|---|
| 18 | 19200 | `0C` | `0.00` | `0.00`, allumé | 19200 n'est pas le débit |
| 19 | 4800 | `0C` | `0.00` | `0.00`, allumé | 4800 n'est pas le débit |
| 20 | **2400** | `0C` | `0.00` | **`0.00` disparu** | **2400 est le débit** |
| 21 | 2400 | `4C 43 4F 53 20 32 34 30 30` (texte nu) | noir | noir | le texte brut ne s'affiche pas : jeu de commandes requis |
| 22 | **2400** | `1B 40` (`ESC @`) | noir | **`0.00` revenu** | **`ESC @` réveille l'afficheur** |
| 23 | **2400** | `1B 51 41` + `LCOS 2400` + `0D` | `0.00` | **`2400`** | **cd5220 est le jeu de commandes.** Les lettres sont ignorées, les chiffres affichés |
| 24 | **2400** | `1B 51 41` + `123.45` + `0D` | `2400` | **`123.45`** | **chaîne complète, point décimal compris** |
| 25 | 2400 | `1B 51 41` + `0123456789` + `0D` | `123.45` | **`23456789`** | **largeur 8**, débordement coupé par la gauche — la convention de `caler()` |
| 26 | 2400 | `1B 51 41` + `1 500,00` + `0D` | `23456789` | **`150000`** | **la virgule et l'espace ne sont PAS rendus** — montant faux à l'écran |
| 27 | 2400 | `1B 51 41` + `1500.00` + `0D` | `150000` | **`1500.00`** | la forme corrigée est bonne |
| 28 | 2400 | `1B 51 41` + `_1500.00` + `0D` | `1500.00` | `1500.00`, placé autrement | l'espace de tête occupe une position |
| 29 | 2400 | `1B 51 41` + `__1500.00` + `0D` | idem 28 | **identique à 28** | neuf caractères tronqués aux huit derniers : **l'écran fait 8 positions, point inclus** |
| 30 | 2400 | `1B 51 41` + `7` + `0D` | — | **`7` tout à droite** | **l'afficheur cale à droite de lui-même** |
| 31 | 2400 | `cmd /c mode COM2: BAUD=2400 …` puis `1B 51 41` + `__123.45` + `0D` | `7` | **`123.45`** | **la chaîne de LCOS est validée de bout en bout** |

**Essai 31 — la chaîne complète de LCOS, reproduite à l'identique.** Tous les
essais précédents passaient par `SerialPort` de .NET, alors que LCOS pose la
vitesse par `cmd /C mode`. Cette consigne n'avait donc jamais été vérifiée
sur COM2, et son échec serait passé inaperçu : `envoyer()` l'ignore avec
`let _ =`. Windows l'accepte et confirme **2400, None, 8, 1, DTR ON, RTS ON,
CTS et DSR désactivés**. La trame exacte de `trame("123,45", "cd5220", 8)`
envoyée ensuite sur `\\.\COM2` affiche `123.45`.

**Défaut à corriger, trouvé à l'essai 26** : LCOS formate les sommes à la
française, `1 500,00`. L'afficheur ignore la virgule et l'espace, et affiche
`150000` — le client lit cent cinquante mille au lieu de mille cinq cents.
Le **point** décimal, lui, est rendu (essai 24). Il faut donc convertir la
virgule en point et retirer les espaces avant l'envoi.

**Découverte de l'essai 23 : l'afficheur est NUMÉRIQUE.** `LCOS 2400` s'est
affiché `2400` : les cinq caractères alphabétiques ont été ignorés, les
quatre chiffres retenus. D'où le `0.00` du départ — ce n'est pas un message
d'initialisation, c'est **un montant à zéro**. Ça convient parfaitement à
LCOS, qui n'envoie qu'une somme.

**Point de méthode, relevé par l'utilisateur à l'essai 20** : sur cet
afficheur, « éteint » et « allumé mais vide » sont **indiscernables à
l'œil** — les caractères sont lumineux sur fond noir. Le témoin `0.00` ne
sépare donc pas ces deux états. Seul un affichage réussi les sépare : si du
texte apparaît, l'écran était vide et non éteint.

## Outils

```
outils\sonder_afficheur_com2.ps1 -Debits <n> -Trame efface    # 0C seul
outils\sonder_afficheur_com2.ps1 -Debits <n> -Trame texte     # texte seul, sans 0C
outils\sonder_afficheur_com2.ps1 -Debits <n>                  # 0C puis texte
```

Le script est volontairement doux : une seule ouverture de port par débit,
maintenue ouverte pendant l'observation. Ne jamais revenir à une boucle
d'ouvertures rapides.
