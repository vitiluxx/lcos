# Guide editeur LCOS 1.9.8

Reserve a l editeur et a l installateur : emission des licences, reglage de
l afficheur client, inventaire du materiel. Le guide du commercant est
`recherches/guide_utilisation_lcos.md`.

Guide ecrit sans accents : le meme texte s affiche correctement sous Windows 10,
dans un editeur, dans une console et sur un ticket.

Convention de ce guide :

- **Emplacement** = le chemin exact pour trouver l option dans le logiciel.
- `-->` = l etape suivante obligatoire, dans cet ordre.
- *Pourquoi* = la raison courte, quand elle evite une erreur.

---

## 1. Emettre une licence pour un poste

Trois commandes, rien de plus. Tout se passe sur **ta** machine ; chez le client, un
copier-coller. Aucune connexion internet d un bout a l autre.

### Ce qu il te faut

- **Dossier ou lancer les commandes** : `C:\projets_codes\lcos\src-tauri`
- **Cle privee de signature** :
  `C:\Users\conne\Documents\LCOS\cle_privee_licence_lcos.txt`
- **Nom du commerce** : donne par le client
- **Numero d emission** : le suivant de ton registre : `0001`, `0002`, `0003`...
- **Empreinte du poste** : relevee a l etape 1 ; facultative

Sans la cle privee, plus aucune licence ne peut etre emise. Copie-la sur une cle USB
rangee ailleurs. Jamais dans le depot, jamais dans un courriel.

### Etape 1 - Relever l empreinte du poste

Au telephone, fais lire au client :

```
LCOS --> icone a trois barres --> Parametrage --> code du gerant --> Licence
```

La ligne `Empreinte de ce poste` : quatre groupes, par exemple `8A1F-9F27-72BE-F939`.
Chiffres et lettres `A` a `F` uniquement. Fais repeter deux fois.

Essai deja termine : lecran Licence s affiche seul au demarrage, l empreinte y est, le
code du gerant n est pas demande.

Pour une licence valable sur nimporte quel poste, saute cette etape et retire `--poste`
a l etape 3.

### Etape 2 - Ouvrir la console

Menu Demarrer --> `PowerShell`, puis :

```
cd C:\projets_codes\lcos\src-tauri
```

C est le dossier qui contient `Cargo.toml`. Ailleurs, la commande repond
`could not find Cargo.toml`.

### Etape 3 - Fabriquer la cle

Une seule ligne :

```
cargo run --quiet --bin lcos_licence -- generer --cle-privee @C:\Users\conne\Documents\LCOS\cle_privee_licence_lcos.txt --numero 0001 --titulaire "Epicerie station service de lamadji" --jours 18250 --poste DBC1-68B3-CF9C-CD48 --sortie C:\Users\conne\Documents\LCOS\licence_0001_EPICERIE_LAMADJI.txt
```

A changer a chaque client :

- `--numero 0002` : le numero suivant de ton registre
- `--titulaire "Boutique Chez Ali"` : le nom du commerce, **entre guillemets**
- `--jours 365` : la duree accordee
- `--poste 8A1F-9F27-72BE-F939` : l empreinte de l etape 1
- `--sortie C:\...\licence_0002.txt` : le fichier ou ecrire la cle

Le `@` devant le chemin signifie *lis la valeur dans ce fichier* : la cle privee n entre
pas dans l historique de la console.

Reponse attendue :

```
licence pour Boutique Chez Ali (numero 0002)
emise le 2026-11-01 - valable jusqu au 2027-11-01
poste : 8A1F-9F27-72BE-F939

LCOS1.eyJudW1lcm8iOiIwMDAyIiwidGl0dWxhaXJlIjoiQm91dGlxdWUgQ2hleiBBbGki...

ecrit dans C:\Users\conne\Documents\LCOS\licence_0002.txt
```

Relis ces trois premieres lignes avant d envoyer : le nom, la date de fin, le poste. Si
elles s affichent, la signature a deja ete verifiee par l outil : la cle est bonne.

La premiere commande de la journee prend une a deux minutes, le temps de compiler
l outil. Les suivantes repondent tout de suite.

### Etape 4 - Envoyer et faire coller

Le fichier `licence_0002.txt` contient un seul texte, commencant par `LCOS1.`. WhatsApp,
courriel ou cle USB, au choix : il n y a aucun secret dedans. Chez le client :

```
LCOS --> Parametrage --> Licence --> champ Cle de licence --> bouton Activer la licence
```

La caisse rouvre immediatement. Pas de redemarrage, pas de reinstallation, pas de reseau.
Caisse deja fermee : l ecran Licence est le premier a l ouverture, le champ est sous les
yeux du client.

### Verifier une cle chez toi (depannage)

```
cd C:\projets_codes\lcos\src-tauri
cargo run --quiet --bin lcos_licence -- verifier @C:\Users\conne\Documents\LCOS\licence_0002.txt
```

```
licence valide
  numero     : 0002
  titulaire  : Boutique Chez Ali
  emise le   : 2026-11-01
  validite   : jusqu au 2027-11-01
  poste      : 8A1F-9F27-72BE-F939
```

Un client dit que sa cle est refusee : relis-la ici et compare la ligne `poste` avec
l empreinte que sa caisse affiche.

### Les variantes de la commande

- **Une licence sans fin** : `--perpetuelle` au lieu de `--jours 365`
- **Une licence valable sur tout poste** : retirer `--poste 8A1F-...` entierement
- **Une periode de reflexion** : `--jours 30`
- **Imposer la date d emission** : ajouter `--emise-le 2026-11-01`
- **Juste afficher la cle, sans fichier** : retirer `--sortie C:\...`

### Les cas courants

- **Renouveler une licence arrivee a terme** : emettre une nouvelle licence avec le
  numero suivant. Le client la colle par-dessus l ancienne, rien a supprimer.
- **Deuxieme caisse dans le meme commerce** : une licence par caisse, avec l empreinte
  de chacune, et deux numeros differents.
- **Le client a reinstalle Windows, ou supprime `lcos.db`** : L empreinte a change.
  Redemande-la et emets une nouvelle licence.
- **Le client dit que la cle est refusee** : fais-lui recoller la cle entiere, de
  `LCOS1.` jusqu au dernier caractere ; puis relis-la avec `verifier` et compare le
  poste.
- **Retirer une licence d un poste** : chez le client :
  `LCOS --> Parametrage --> Licence --> bouton Retirer la licence`. La caisse se
  referme, les donnees restent.

### Tiens un registre, a la main

Le logiciel ne garde aucune trace de qui a recu quoi. Le numero est le seul lien entre
une licence et un client. Un fichier texte suffit :

```
C:\Users\conne\Documents\LCOS\registre_licences.txt
```

```
0001 ; 2026-10-24 ; Boutique Chez Ali   ; 8A1F-9F27-72BE-F939 ; 365 j ; fin 2027-10-24
0002 ; 2026-11-01 ; Alimentation Fatou  ; tout poste          ; 30 j  ; fin 2026-12-01
```

### Une commande a ne jamais relancer

```
cargo run --bin lcos_licence -- cles
```

Elle fabrique une **nouvelle** paire de cles. Elle ne sert qu une fois dans la vie du
projet, et c est deja fait. La relancer et coller la nouvelle cle publique dans le code
rendrait d un coup **toutes les licences deja livrees invalides**, sur toutes les caisses
installees.

---

## 2. Installer et regler l afficheur client

**Emplacement** : `LCOS --> Parametrage --> Peripheriques --> Afficheur client`

Ce que le commercant en voit au quotidien est decrit dans son guide,
chapitre 7 ter. Ce chapitre-ci ne traite que du reglage.

### Savoir par ou il est branche

Deux branchements, et un seul geste pour savoir lequel :

```
Parametrage --> Peripheriques --> Materiel detecte sur ce poste
--> bouton Relever le materiel --> tape "com" dans la recherche
```

- **Une ligne avec un `COM` dans la colonne `Port`** : l afficheur a son propre port
  serie, meme si la prise est de l USB. A regler sur `Son propre port serie (COM)`
- **Aucune ligne avec un `COM`** : l afficheur est chaine sur la carte de l imprimante
  thermique : il n a pas de port a lui. A regler sur `Derriere l imprimante du ticket`

C est le cas des caisses tout-en-un : la carte de l imprimante porte une prise pour
l afficheur, et rien n apparait dans les ports de Windows. LCOS envoie alors la somme au
spouleur de l imprimante du ticket, precedee de la commande qui donne la main a
l afficheur et suivie de celle qui la rend au rouleau.

### Le regler

```
case Envoyer la somme sur l afficheur client --> Branchement --> (Port, Vitesse)
--> Largeur --> bouton Enregistrer --> bouton Afficher 123,45 pour voir
```

- `Branchement` : voir la liste juste au-dessus
- `Port` : le `COM` trouve dans l inventaire. Absent quand l afficheur passe par l
  imprimante
- `Vitesse` : `9600` dans la quasi totalite des cas. Des caracteres au hasard a
  l ecran : essaie les autres vitesses
- `Protocole` : voie serie seulement. Six jeux de commandes : `Texte brut`, `CD5220`,
  `ESC/POS`, `AEDEX`, `UTC Enhanced`, `ADM 787 / 788`. Aucun afficheur ne dit lequel
  il parle : essaie-les dans l ordre de la liste en regardant l ecran du client. Si les
  six restent muets, change la vitesse plutot que de recommencer
- `Largeur` : nombre de caracteres par ligne, `20` d ordinaire, `8` au minimum

**Voie imprimante : choisis l imprimante du ticket d abord** (panneau juste
au-dessus ; chapitre 7 bis du guide du commercant). Sans imprimante retenue, LCOS
refuse d allumer l afficheur, parce qu il ne saurait pas par ou envoyer les octets.

Le bouton `Afficher 123,45 pour voir` envoie une somme d essai sans rien enregistrer :
regarde l ecran arriere pendant que tu cliques. `Effacer l ecran` le rend a blanc.

**La vitesse du port est reposee par LCOS a chaque lancement**, avant le premier envoi.
Windows ne la garde pas d un demarrage a l autre : jusqu a la 1.9.2, la caisse redemarree
parlait a l afficheur a la vitesse par defaut du port, et l ecran restait muet pendant
les ventes alors que le bouton d essai, lui, le reveillait. Rien a resaisir apres un
demarrage.

#### Reglages etablis sur la caisse P70L

Mesures sur le materiel, trente essais consignes dans
`recherches/journal_afficheur.md`. A reporter tels quels sur une caisse de ce modele :

- `Branchement` : `Son propre port serie (COM)`
- `Port` : **COM2**
- `Vitesse` : **2400** bauds
- `Protocole` : **CD5220**
- `Largeur` : **8**

Cet afficheur est **numerique** : huit positions, chiffres et point decimal seulement.
Il ignore les lettres sans rien signaler, et il cale les chiffres a droite tout seul. Le
`0.00` qu il montre au repos n est pas un message d accueil : c est un montant a zero.

**Ecran noir ?** Le reveil est `ESC @` a 2400 bauds sur COM2, c est-a-dire le bouton
`Afficher 123,45 pour voir` apres avoir pose ces reglages. Avant de connaitre ce remede,
le seul moyen etait de debrancher le cordon secteur une vingtaine de secondes : un
redemarrage de Windows ne suffit pas, il ne coupe pas l alimentation de l afficheur.

**Deux choses a ne jamais faire sur ce port :** installer une imprimante du spouleur sur
`COM2` (elle reserve le port, et il faut redemarrer le poste pour le liberer), et
enchainer les essais en rafale (cent ouvertures de port en seize secondes eteignent
l afficheur).

### Pourquoi la somme part avec un point decimal

Les sommes partent avec un **point** decimal et sans separateur de milliers
(`1500.00`), parce que ces afficheurs ignorent la virgule et l espace : `1 500,00` s y
lisait `150000`, soit cent cinquante mille au lieu de mille cinq cents.

### Les refus possibles

- `port de l afficheur : COM1 a COM256 attendu`
  - Raison : le port n est pas un port serie
  - Action : reporte le `COM` de l inventaire
- `vitesse refusee par Windows`
  - Raison : le port n existe pas, ou un autre logiciel le tient
  - Action : verifie le port dans l inventaire, ferme l autre logiciel
- `choisis d abord l imprimante du ticket`
  - Raison : voie imprimante, mais aucune imprimante retenue
  - Action : `Parametrage --> Peripheriques --> Imprimante du ticket --> Detecter`
- `COMn ne repond pas`
  - Raison : appareil debranche ou eteint
  - Action : rebranche, puis `Afficher 123,45 pour voir`

---

## 3. Materiel detecte sur ce poste

**Emplacement** : `LCOS --> Parametrage --> Peripheriques --> Materiel detecte sur ce poste`

```
bouton Relever le materiel  (la lecture prend quelques secondes)
```

La meme liste que le gestionnaire de peripheriques de Windows, lisible sans quitter
LCOS : **tout ce qui est branche** (USB, serie, Bluetooth) et **tout ce qui est integre**
(carte mere, PCI, disques, ecran tactile).

- `Appareil` : le nom donne par Windows
- `Famille` : le genre : `Ports`, `Monitor`, `HIDClass`, `USB`, `Printer`...
- `Liaison` : comment l appareil tient a la machine : `USB (branche)`, `PCI (interne)`,
  `HID`, `ACPI (carte mere)`...
- `Port` : le `COM` quand l appareil est sur un port serie. C est la colonne a lire pour
  l afficheur client
- `Etat` : `en service`, `en defaut`, `inconnu`
- `Fabricant` : tel que Windows le connait

Le champ de recherche et la liste des familles filtrent la liste : tape `com` pour les
ports serie, `usb` pour ce qui est branche, le nom du fabricant de la douchette pour la
retrouver.

*A quoi ca sert, concretement :* trouver le port de l afficheur client, verifier que la
douchette est bien vue par Windows, savoir si l imprimante integree est en defaut, et
cela sans ouvrir une seule fenetre de Windows sur une caisse tactile.

*Pourquoi la liste est parfois ecourtee :* un poste Windows declare plus de mille
elements, pilotes systeme compris. LCOS en garde les 500 premiers par famille et le dit
sous la liste.

---

## 4. Depannage reserve a l installation

- **L afficheur client reste eteint**
  - Cause : la case n est pas cochee, ou le branchement est faux
  - Action : `Parametrage --> Peripheriques --> Afficheur client`, puis
    `Afficher 123,45 pour voir`
- **Windows ne montre aucun second ecran**
  - Cause : normal : l afficheur client n est pas un moniteur
  - Action : il se regle dans `Peripheriques --> Afficheur client`, pas dans l
    affichage de Windows
- **Aucun `COM` dans le materiel detecte**
  - Cause : l afficheur est chaine derriere l imprimante
  - Action : branchement `Derriere l imprimante du ticket`
- **L afficheur client montre des caracteres au hasard**
  - Cause : mauvaise vitesse
  - Action : essaie une autre vitesse, 9600 d abord
- **L ecran repond au bouton d essai mais reste muet pendant les ventes**
  - Cause : vitesse du port perdue au redemarrage du poste, jusqu a la 1.9.2
  - Action : installe la 1.9.3 ou plus recent ; en attendant, un passage par
    `Enregistrer` ou `Afficher 123,45 pour voir` la repose pour la session
- **`Materiel detecte` repond `lecture impossible`**
  - Cause : le service materiel de Windows etait occupe
  - Action : reappuie sur `Relever le materiel` : LCOS fait deja un second essai tout
    seul
