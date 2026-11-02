# Guide d utilisation LCOS 1.9.5

Guide ecrit sans accents : le meme texte s affiche correctement sous Windows 10,
dans un editeur, dans une console et sur un ticket.

Ce guide est celui du commercant. L emission des licences, le reglage de l afficheur
client et l inventaire du materiel sont dans `recherches/guide_editeur_lcos.md`.

Convention de ce guide :

- **Emplacement** = le chemin exact pour trouver l option dans le logiciel.
- `-->` = l etape suivante obligatoire, dans cet ordre.
- *Pourquoi* = la raison courte, quand elle evite une erreur.

---

## 1. Trois ecrans, derriere une icone

La barre d onglets a disparu de la page : la navigation tient dans une **icone a trois
barres**, en haut a droite, juste a cote de `version 1.9.5`.

**Emplacement** : `LCOS --> entete en haut a droite --> icone a trois barres`

Un clic ouvre la liste, un clic sur une ligne change d ecran, un clic ailleurs ou la
touche `Echap` referme la liste.

- **Caisse**
  - Emplacement : `icone --> Caisse`
  - A quoi il sert : encaisser les clients. C est l ecran de travail.
- **Session**
  - Emplacement : `icone --> Session`
  - A quoi il sert : ouvrir et fermer la journee d un caissier, relire les ventes
    enregistrees, et les annuler si le gerant l autorise.
- **Parametrage**
  - Emplacement : `icone --> Parametrage`
  - A quoi il sert : tout creer, tout modifier, tout consulter. Protege par le code du
    gerant.

Tout le reste vit dans le Parametrage, en sous-onglets, chacun ouvert par
`LCOS --> Parametrage --> <nom du sous-onglet>` :

- **Commerce et ticket** : nom du commerce, ticket, largeur du papier.
- **Devise et decimales** : devise, et valeurs apres la virgule.
- **Impression des recus** : sortir ou non une feuille apres chaque vente.
- **Date et heure** : horloge de la caisse et droits administrateur.
- **Caissiers** : comptes des caissiers.
- **Rayons** : familles de produits.
- **Produits** : catalogue, prix, codes-barres.
- **Code du gerant** : code et nom du gerant.
- **Licence** : etat du droit d usage, empreinte du poste, insertion d une licence.
- **Export et import** : copier la programmation d une caisse a l autre, sauvegarder,
  restaurer.
- **Rapports** : ventes, rapport Z, remise a zero.
- **Peripheriques** : douchette et imprimante du ticket.
- **Diagnostic** : verifier que l installation est saine.

*Pourquoi :* la barre d onglets prenait une ligne entiere en haut de chaque ecran. Sur
l ecran de la caisse, cette ligne vaut deux rangees de la calculatrice. Les rapports, les
peripheriques et le diagnostic etant des affaires de gerant, ils sont desormais derriere
son code.

---

## 2. Installer ou mettre a jour

Fichier d installation :

```
C:\projets_codes\lcos\src-tauri\target\release\bundle\nsis\LCOS_1.9.5_x64-setup.exe
```

1. Copie ce fichier sur une cle USB.
2. Sur la caisse : double-clic dessus.
3. Installe par-dessus la version precedente. **Ne desinstalle rien.**

*Pourquoi :* la mise a jour conserve la base de donnees. Desinstaller d abord ferait
perdre produits, caissiers et ventes.

Apres installation :

- Programme : `C:\Users\<utilisateur>\AppData\Local\LCOS\lcos.exe`
- Raccourci : menu Demarrer --> `LCOS`
- Base de donnees : `C:\Users\<utilisateur>\AppData\Roaming\org.lcos.caisse\lcos.db`

Aucune connexion internet n est utilisee, ni a l installation ni a l usage.

---

## 2 bis. Droit d usage : essai gratuit puis licence

**Emplacement** : `LCOS --> Parametrage --> Licence`, ou directement le badge en haut a
droite de l entete (`essai : 7 j`), qui ouvre le meme ecran.

### L essai gratuit

Une caisse neuve fonctionne pendant **sept jours**, sans rien saisir : installe,
programme, vends, juge. Le badge de l entete decompte les jours restants.

Tout fonctionne pendant l essai : les ventes, les tickets, les rapports Z, l annulation,
la douchette, l imprimante, l afficheur client. Quatre choses seulement sont plafonnees,
le temps de juger le logiciel :

- **Caissiers** : 1
- **Rayons** : 1
- **Produits** : 6
- **Export et import du parametrage** : fermes

Le bouton `Creer` se grise des que le plafond est atteint, et un avertissement le dit
au-dessus du formulaire. L insertion d une licence leve les quatre plafonds dans la
seconde, sans rien resaisir : les comptes, rayons et produits deja crees restent en
place.

*Pourquoi ces plafonds :* une boutique entiere programmee pendant l essai n a plus
aucune raison de prendre une licence. Un caissier, un rayon et six produits suffisent a
essayer la caisse pour de vrai, ventes et rapports compris. Et l export, lui, recopierait
une programmation complete d un poste a l autre : ce serait la porte ouverte pour peupler
une caisse sans licence en un seul fichier.

### Apres l essai

Sans licence, le huitieme jour, la caisse se ferme : LCOS n affiche plus que l ecran
`Caisse fermee : licence requise`, la navigation ne mene plus ailleurs, et le logiciel
refuse d ouvrir une session comme d encaisser.

**Rien n est efface.** Produits, caissiers, reglages, ventes et rapports Z restent en
place et reviennent intacts des l insertion d une licence.

### Inserer une licence

```
ecran Licence --> champ Cle de licence --> bouton Activer la licence
```

- La cle est un texte qui commence par `LCOS1.`. Colle-la : les espaces et les retours a
  la ligne n ont pas d importance.
- La caisse rouvre **immediatement**, sans redemarrage et sans connexion internet.
- `Empreinte de ce poste` : un code en quatre groupes, par exemple `8A1F-9F27-72BE-F939`.
  Donne-le a l editeur avec le nom du commerce : il sert a emettre une licence qui ne
  vaut que pour cette caisse. Une licence emise sans empreinte fonctionne sur n importe
  quelle installation.

- `signature de licence invalide` : cle incomplete, mal recopiee, ou qui ne vient pas de
  l editeur
- `licence de type ... inconnu` : licence d un format plus recent : mets LCOS a jour
- `licence emise pour le poste ...` : licence liee a une autre caisse
- `licence expiree le ...` : demande une licence a jour

*Pourquoi reculer l horloge ne sert a rien :* LCOS retient le jour le plus avance qu il
ait jamais vu. Remettre la date en arriere ne rend aucun jour d essai. C est volontaire
sur un logiciel qui sait regler cette horloge lui-meme.

*Une licence se verifie hors ligne, pour toujours :* elle porte la signature de
l editeur, et LCOS ne connait que la cle publique qui la verifie. Aucun appel reseau,
aucun serveur a joindre, aujourd hui comme dans dix ans.

---

## 3. Mise en route, dans cet ordre strict

```
Proteger la caisse --> Rayons --> Produits --> Caissiers --> Date et heure --> Session --> Caisse
```

*Pourquoi cet ordre :* un produit exige un rayon, une session exige un caissier, une
vente exige une session. Sauter une etape bloque la suivante.

### 3.1 Proteger la caisse (une seule fois)

**Emplacement** : `LCOS --> Parametrage` (l ecran s ouvre tout seul au premier demarrage)

1. `Nom du gerant` --> ton nom.
2. `Code secret` --> de 4 a 8 chiffres.
3. `Confirmation` --> le meme code.
4. Bouton `Creer le compte gerant`.

*Pourquoi :* ce code protege tout le menu Parametrage, la remise a zero des ventes et
la fermeture d une session en l absence du caissier. Il n est jamais enregistre en
clair et ne peut pas etre recupere : note-le ailleurs.

Le compte gerant ne peut pas etre supprime. Il peut etre renomme et son code change.

### 3.2 Entrer dans le Parametrage les fois suivantes

**Emplacement** : `LCOS --> Parametrage --> champ Code du gerant --> bouton Deverrouiller`

- Pour refermer : `LCOS --> Parametrage --> bouton Verrouiller` (en haut a droite).
- L acces se referme tout seul apres 15 minutes sans activite.
- Apres 5 codes faux, l acces est bloque 60 secondes.

---

## 4. Parametrage, section par section

### 4.1 Commerce et ticket

**Emplacement** : `LCOS --> Parametrage --> Commerce et ticket`

- `Nom du commerce` : titre imprime en haut du ticket. S affiche aussi dans l entete du
  logiciel
- `Ticket active` : produit ou non un ticket. decoche pour vendre sans ticket
- `Largeur (colonnes)` : largeur du papier en caracteres. **32** pour le rouleau 58 mm
  de la caisse
- `Mention d entete` : ligne sous le nom du commerce. exemple : `Merci de votre visite`
- `Mention de pied` : derniere ligne du ticket. exemple : `A bientot`

Terminer par : `bouton Enregistrer`.

*Pourquoi la largeur :* le ticket est mis en page caractere par caractere. Une largeur
fausse coupe les prix a droite.

### 4.2 Devise et decimales

**Emplacement** : `LCOS --> Parametrage --> Devise et decimales`

```
champ Devise --> liste Valeurs apres la virgule --> bouton Enregistrer
```

- `Devise` : texte affiche a cote des totaux. exemple : `EUR`, `FCFA`, `XOF`, `USD`
- `Valeurs apres la virgule` : `Activees : 2 decimales` ou
  `Desactivees : entiers pleins`. choix propose automatiquement selon la devise

- Devise **FCFA**, `CFA`, `XOF`, `XAF` : LCOS propose de lui-meme `Desactivees`. Tape
  `FCFA` dans `Devise`, sors du champ, la liste se place seule.
- Decimales desactivees : plus aucune virgule nulle part. Le total affiche `500`, le
  ticket imprime `500`, le rapport Z imprime `500`, et la touche `,` disparait de la
  calculatrice.
- Decimales activees : `500,00` partout, comme avant.

Le `bouton Enregistrer` du panneau `Ticket de caisse` enregistre aussi ces deux champs.

*Pourquoi :* une somme en franc CFA n a pas de centime. Une virgule a l ecran d une
caisse en FCFA est une faute de lecture a chaque vente, pas un detail de presentation.

### 4.3 Impression des recus

**Emplacement** : `LCOS --> Parametrage --> Impression des recus`

```
case Imprimer le recu apres chaque vente --> bouton Appliquer
```

Le badge du panneau indique l etat courant : `activee` ou `desactivee`.

- `activee` : le ticket part a l imprimante et le panneau du ticket s affiche a l ecran.
- `desactivee` : aucune feuille, aucun panneau : une confirmation apparait **une
  seconde** au centre de l ecran, puis disparait.

- La confirmation porte la somme a rendre quand il y en a une :
  `Vente encaissee - a rendre 500`.
- Un appui sur `Entree` la fait partir aussitot, sans attendre la seconde complete.
- **Le rapport Z n est jamais concerne** : il s imprime toujours, meme avec l impression
  des recus desactivee. Ce reglage ne touche que l ecran Caisse.

*Pourquoi un bouton a part :* c est un geste quotidien. Couper les recus un jour de
penurie de rouleau ne doit pas obliger a rouvrir tout le reglage du ticket.

### 4.3 bis Annulation des ventes

**Emplacement** : `LCOS --> Parametrage --> Commerce et ticket --> Annulation des ventes`

```
case Autoriser l annulation d une vente deja encaissee --> bouton Appliquer
```

Le badge du panneau dit l etat courant : `activee` ou `desactivee`. **A l installation,
l option est fermee.**

- `desactivee` : les ventes se lisent, rien ne s annule. Aucune croix sur les lignes, la
  touche `Suppr` ne fait rien.
- `activee` : une croix apparait sur chaque ligne, et la touche `Suppr` agit sur la
  vente choisie. Un code est demande a chaque annulation.

Ce que fait une annulation, en une fois : la vente quitte le journal, le stock des
articles revient, les totaux de la session et du rapport Z redescendent, et une trace
nominative part dans `LCOS --> Parametrage --> Rapports --> Ventes annulees`.

Ce qu une annulation ne fait pas : **rendre le numero de ticket**. Le ticket annule
reste consomme, le suivant prend le numero d apres. Le trou dans la suite des numeros
s explique dans le tableau des ventes annulees.

*Pourquoi ferme par defaut :* une caisse ou n importe qui efface une vente est une
caisse sans comptabilite. C est au gerant d ouvrir cette porte, en sachant que chaque
annulation laisse le nom de qui l a faite.

### 4.4 Date et heure

**Emplacement** : `LCOS --> Parametrage --> Date et heure`

1. `Date (AAAA-MM-JJ)` --> exemple `2026-10-24`.
2. `Heure (HH:MM)` --> exemple `08:30`.
3. Bouton `Appliquer`.

Si Windows affiche une demande d autorisation : accepte-la. LCOS redemande l horloge
dans un processus administrateur, sans qu il faille relancer le logiciel.

Si Windows refuse quand meme : bouton `Ouvrir le panneau Windows` --> regle la date dans
Windows --> reviens dans LCOS.

*Pourquoi :* changer l horloge du poste demande les droits administrateur Windows. Une
horloge fausse date faussement toutes les ventes et tous les rapports.

#### Supprimer la demande d autorisation une fois pour toutes

**Emplacement** : `LCOS --> Parametrage --> Date et heure --> panneau Droits administrateur de LCOS`

```
bouton Autoriser LCOS en administrateur --> accepter la fenetre Windows -->
fermer LCOS --> relancer par le raccourci du bureau LCOS (administrateur)
```

LCOS pose une tache planifiee `LCOS_Administrateur` au niveau le plus eleve et cree un
raccourci sur le bureau qui la declenche. Lance par ce raccourci, LCOS demarre deja
administrateur et Windows n affiche plus rien.

Le badge du panneau indique l etat du lancement en cours : `administrateur` ou
`droits ordinaires`. Bouton `Retirer l autorisation` pour revenir en arriere.

*Pourquoi un raccourci separe :* Windows ne laisse pas un programme ordinaire s elever
tout seul. Une tache planifiee est la seule voie qui supprime la demande a chaque
demarrage au lieu de la repeter.

### 4.5 Caissiers

**Emplacement** : `LCOS --> Parametrage --> Caissiers`

**Creer** : `champ Nom --> champ Code secret (4 a 8 chiffres) --> case Compte actif --> bouton Creer`

**Modifier** : `colonne de droite --> ligne du caissier --> bouton Modifier --> corriger --> bouton Enregistrer`

- Laisser `Code secret` **vide** en modification : l ancien code est conserve.
- Remplir `Code secret` en modification : le code est remplace.

**Desactiver** : `bouton Modifier --> decocher Compte actif --> bouton Enregistrer`

**Supprimer** : `ligne du caissier --> bouton Supprimer --> bouton Confirmer ?`

*Pourquoi deux clics :* la suppression est definitive, le second clic evite l accident.

Un caissier qui a deja des ventes **ne peut pas** etre supprime : desactive-le. Il
disparait de l ecran d ouverture de session mais ses ventes restent dans les rapports.

*Version d essai :* un seul caissier. Le bouton `Creer` est grise au-dela, et
l avertissement renvoie a la licence (chapitre 2 bis).

### 4.6 Rayons

**Emplacement** : `LCOS --> Parametrage --> Rayons`

**Creer** : `champ Libelle --> champ Ordre d affichage --> bouton Creer`

- `Libelle` : nom du rayon. exemple : `Boissons`, `Epicerie`
- `Ordre d affichage` : position dans la caisse. `0` en premier, puis `1`, `2`...
- `Rayon actif` : visible ou masque. decocher masque le rayon sans rien perdre

Un rayon qui contient des produits ne peut pas etre supprime : vide-le d abord, ou
decoche simplement `Rayon actif`.

*Version d essai :* un seul rayon.

### 4.7 Produits

**Emplacement** : `LCOS --> Parametrage --> Produits`

Chaine complete :

```
champ Libelle --> liste Rayon --> champ Code-barres --> champ Prix --> champ Stock --> bouton Creer
```

- `Libelle` : nom imprime sur le ticket. 2 a 60 caracteres
- `Rayon` : rayon du produit. obligatoire. Cree le rayon avant le produit
- `Code-barres` : code lu par la douchette. **Clique dans le champ puis scanne l
  etiquette**
- `Prix` : prix de vente. `1500` ou `1500,50`. Sans decimale : `1500` seulement
- `Stock` : `NON` ou un nombre. **`NON` par defaut** : le produit se vend sans aucun
  stock
- `Produit en vente` : vendable ou non. decoche retire le produit de la caisse

**Le champ `Stock` vaut `NON` sur une fiche neuve.** Laisse-le tel quel : le produit se
vend immediatement, sans quantite a saisir et sans decompte. Pour suivre un stock, tape
un nombre a la place de `NON` : la quantite est alors retiree a chaque vente.

*Pourquoi :* un commerce ne compte pas tout. Exiger un stock avant de pouvoir vendre
bloquait la mise en service du catalogue.

**Modifier** : `tableau de droite --> ligne du produit --> bouton Modifier --> corriger --> bouton Enregistrer`

**Chercher** : `champ Chercher un produit` (au-dessus du tableau), sur le nom ou le code.

*Produit sans code-barres :* laisse `Code-barres` vide. Le produit se vend alors au
toucher, depuis la grille de l onglet Caisse.

*Pourquoi scanner dans le champ :* le code recopie est exactement celui que la
douchette enverra en caisse. Le taper a la main expose a une faute d un chiffre.

*Version d essai :* six produits au maximum.

### 4.8 Code du gerant

**Emplacement** : `LCOS --> Parametrage --> Code du gerant`

**Changer le code** : `champ Code actuel --> champ Nouveau code --> champ Confirmation --> bouton Changer le code`

**Changer le nom** : `champ Nom --> bouton Renommer`

### 4.9 Export et import

**Emplacement** : `LCOS --> Parametrage --> Export et import`

Un seul fichier, au format `.json`, lisible dans n importe quel editeur de texte et relu
par LCOS. Deux portees, jamais melangees :

- **Parametrage seul**
  - Ce que le fichier contient : reglages, comptes (gerant et caissiers), rayons,
    produits avec prix et codes-barres
  - A quoi ca sert : programmer une caisse sur un poste, puis porter le tout sur la
    caisse. Monter une seconde caisse a l identique.
- **Avec les ventes**
  - Ce que le fichier contient : le parametrage, plus les compteurs, les sessions, les
    ventes et les rapports Z
  - A quoi ca sert : sauvegarder une caisse et la restaurer telle quelle.

Le premier panneau annonce ce que la caisse contient : produits, rayons, comptes,
reglages, ventes en memoire, rapports Z archives.

**Version d essai : l export et l import sont fermes.** Les boutons et les champs sont
grises, un avertissement le rappelle en haut de l ecran, et le reste de la caisse
fonctionne normalement. Insere une licence pour les ouvrir (chapitre 2 bis).

#### Exporter

```
champ Dossier --> bouton Exporter le parametrage seul (ou Exporter avec les ventes)
```

- `Dossier` : le dossier d ecriture. Par defaut `C:\Users\<utilisateur>\Documents\LCOS`.
  **Mets la lettre de la cle USB** (`E:\` par exemple) pour ecrire directement dessus.
- Le fichier est nomme tout seul : `lcos_parametrage_20261024-1803.json`,
  `lcos_complet_20261024-1803.json`. La date et l heure sont dans le nom, donc le tri du
  dossier est deja chronologique.
- La ligne sous les boutons rappelle le chemin exact du dernier fichier ecrit.

#### Importer

```
bouton Relire le dossier --> liste Fichier du dossier --> case de confirmation --> bouton Importer ce fichier
```

Deux facons de designer le fichier, au choix :

1. **La liste `Fichier du dossier`** : elle montre les fichiers `.json` du dossier
   indique dans le panneau `Exporter` juste au-dessus. Le dossier n est pas le bon ?
   Corrige-le la-haut, puis `Relire le dossier`. Le bouton est repete dans le panneau
   d import pour ne pas avoir a remonter.
2. **Le champ `Ou chemin complet du fichier`** : tape le chemin entier, par exemple
   `E:\lcos_parametrage_20261025-1830.json`. Rempli, ce champ **prime sur la liste**.
   C est la voie a prendre quand la cle USB n est pas le dossier d export, ou quand la
   liste reste vide.

- La liste decrit chaque fichier : portee, date de generation, version de LCOS qui l a
  ecrit, taille. Un fichier qui n est pas une sauvegarde LCOS est marque `illisible` et
  sera refuse.
- `Aucun fichier .json dans ce dossier` n est pas une panne : c est qu il n y a rien a
  importer la. La liste reste cliquable, et le champ du chemin complet prend le relais.
- `Importer aussi les ventes, sessions, compteurs et rapports Z` : a cocher uniquement
  pour restaurer une sauvegarde complete sur la meme caisse.
- Le bouton reste grise jusqu a la coche de confirmation.

**L import remplace, il ne fusionne pas.** Les reglages, comptes, rayons et produits de
la caisse sont effaces et remplaces par ceux du fichier, en une seule operation : soit
tout passe, soit rien ne change.

- `une session de caisse est ouverte`
  - Raison : un caissier est en poste
  - Action : `LCOS --> Session --> Fermer la session`
- `N vente(s) en memoire`
  - Raison : importer le parametrage seul effacerait des comptes auxquels ces ventes
    sont rattachees
  - Action : edite un rapport Z (`Parametrage --> Rapports`), puis recommence
- `ce fichier ne contient que le parametrage`
  - Raison : la case des ventes est cochee pour un fichier qui n en a pas
  - Action : decoche `Importer aussi les ventes...`
- `format de fichier N inconnu`
  - Raison : fichier ecrit par une version plus recente de LCOS
  - Action : mets LCOS a jour sur ce poste

*Pourquoi le menu se reverrouille apres un import :* le fichier porte aussi le compte du
gerant. Apres l import, **le code du gerant est celui du fichier**, pas celui de la
caisse. LCOS referme donc le parametrage et demande le code, qui est celui de la caisse
d origine du fichier.

*Le fichier est une clef :* il contient les codes secrets des comptes sous forme
scellee, jamais en clair, mais un scellement reste une empreinte de code. Garde la cle
USB d export comme tu garderais le tiroir-caisse.

*Ce qui ne voyage jamais :* le nom de l imprimante vue par Windows, la signature de la
douchette branchee et les horodatages de detection restent propres a chaque poste. La
caisse d arrivee redetecte son materiel elle-meme.

---

## 5. Utilisation quotidienne

### 5.1 Ouvrir la session du caissier

**Emplacement** : `LCOS --> Session --> Ouvrir une session`

```
pastille du caissier --> champ Code du caissier --> champ Fond de caisse --> bouton Ouvrir la session
```

- `Fond de caisse` : l argent present dans le tiroir avant la premiere vente. Mets `0`
  si tu ne le suis pas.
- Une seule session ouverte a la fois sur le poste.

*Pourquoi :* sans session ouverte, l onglet Caisse reste verrouille. Toutes les ventes
sont rattachees au caissier de la session, c est ce qui permet de lire ses totaux.

### 5.2 Choisir le mode de saisie

**Emplacement** : `LCOS --> Caisse --> tout en bas de la colonne de droite, sous
Annuler la vente --> boutons [Mode scan] [Mode manuel]`

- `Mode scan` : par defaut. Articles etiquetes. La calculatrice saisit la somme recue du
  client, deja posee au total par la touche `Entree`. La touche `*` multiplie la
  derniere ligne lue.
- `Mode manuel` : articles sans etiquette, prix dictes. La calculatrice saisit le prix
  de chaque article.

*Pourquoi :* changer de mode remet la vente a zero. Choisis le mode avant de commencer
a encaisser le client.

### 5.3 Encaisser en mode scan

**Emplacement** : `LCOS --> Caisse --> bouton Mode scan`

```
scanner les articles --> touche Entree --> touche Entree
```

1. **Scanner** : chaque bip ajoute l article, le total se met a jour tout seul. Scanner
   deux fois le meme article passe la quantite a 2. Pour dix exemplaires du meme
   article : un seul bip, puis la touche `*` (voir juste en dessous).
2. **Sans code-barres** : touche la pastille du produit dans la grille en bas, ou tape
   son nom dans `Chercher un article par nom ou code`.
3. **Corriger une ligne** : boutons `-`, `+` et `x` a droite de la ligne du panier.
4. **Premiere `Entree`** : le total se pose tout seul dans `Somme recue du client`,
   en bleu `Compte juste`.
5. **Client qui paie juste** : touche `Entree` une seconde fois, c est encaisse.
6. **Client qui donne plus** : tape le montant recu. Le premier chiffre remplace le
   total pose, et `A rendre` affiche le reliquat en direct. Puis touche `Entree`.
7. Un article scanne apres la premiere `Entree` reste pris en compte : le total pose
   suit le panier.
8. `Echap` efface la somme posee ; un second `Echap` retire la derniere ligne du panier.

*Pourquoi deux `Entree` :* le compte juste est le cas le plus courant de la journee. Il
ne coute plus une seule frappe de chiffre, et la seconde `Entree` reste la validation
qui empeche d encaisser une vente a moitie saisie.

Au doigt, **un seul bouton fait les deux `Entree`** : le grand bouton sous le pave.
Son libelle dit ce que le prochain appui fera, et il change tout seul.

Le ticket s affiche dans la colonne de droite --> bouton `Imprimer` si besoin.

#### Multiplier un article scanne : la touche *

Le code-barres est le meme sur chaque exemplaire : inutile de scanner dix fois la meme
bouteille.

```
scanner l article une fois --> touche * --> taper la quantite --> touche Entree
```

- Le champ `Quantite de l article` apparait, et l aide dessous donne le calcul en
  direct : `Coca 33cl : 500,00 x 4 = 2 000,00`.
- `Entree` (ou la touche `x` du pave, ou `+`) pose la quantite sur la ligne : elle
  **remplace** la quantite, elle ne s y ajoute pas.
- `Echap` annule la multiplication, la ligne garde sa quantite.
- Un nouveau scan pendant la saisie ferme la multiplication : elle visait la ligne
  precedente.
- La touche `*` sans aucun article dit `Scanne d abord un article`.

La touche `x` du pave fait la meme chose au doigt : un premier appui ouvre la quantite,
le second la valide.

### 5.4 Encaisser en mode manuel

**Emplacement** : `LCOS --> Caisse --> bouton Mode manuel`

```
taper le prix --> touche + --> taper le prix suivant --> touche + -->
touche Entree --> taper la somme recue --> touche Entree
```

1. Le champ `Prix de l article` apparait au-dessus de la somme recue, le curseur s y
   place tout seul.
2. Tape le prix --> touche `+` (ou bouton `+ Ajouter l article`, dans le pave, a droite
   des zeros). L article entre dans le panier, le champ se vide pour le suivant.
3. Le libelle au-dessus du gros chiffre affiche `Sommation` : le cumul monte a chaque
   `+`.
4. Touche `Entree` : la sommation est fermee, le libelle repasse a `Total a payer`, le
   curseur saute sur `Somme recue du client` **et ce champ porte deja le total**, en
   bleu `Compte juste`.
5. Client qui paie juste : touche `Entree` tout de suite, c est encaisse. Client qui
   donne plus : tape le montant recu, le premier chiffre remplace le total pose, et
   `A rendre` s affiche. Puis `Entree`.

*Pourquoi deux validations :* la premiere `Entree` fige le total, la seconde encaisse.
Sans cette separation, une frappe de trop validerait une vente incomplete.

*Pourquoi le total est deja pose :* le compte juste est le cas le plus courant de la
journee. Il ne coute plus une seule frappe de chiffre.

#### Multiplier un prix : la touche *

Meme article pris en plusieurs exemplaires, un seul prix a taper :

```
taper le prix --> touche * --> taper la quantite --> touche +
```

- Le champ affiche `1500 x 3`, et la ligne d aide dessous donne le calcul en direct :
  `1 500,00 x 3 = 4 500,00`.
- Touche `+` : une seule ligne entre au panier, prix unitaire et quantite separes, comme
  pour un article scanne trois fois.
- Touche `Entree` a la place de `+` : l article entre **et** la sommation se ferme.
- Quantite maximale : 9999.
- `Echap` avant d avoir fini : la multiplication est retiree, le prix reste.

### 5.5 Le code couleur de la somme recue

A chaque chiffre tape, LCOS compare la somme recue au total a payer :

- **Rouge** : somme recue **inferieure** au total. Sous le champ : `Manque` + ce qui
  reste a encaisser
- **Bleu** : somme recue **egale** au total. Sous le champ : `Compte juste`, aucun
  montant
- **Vert** : somme recue **superieure** au total. Sous le champ : `A rendre` + la
  monnaie a remettre

*Pourquoi :* la monnaie a rendre n apparait que dans le cas vert. Tant que le champ est
rouge, le bouton `Encaisser` reste grise et la vente ne part pas.

- `0` a `9` : chiffres de la somme recue, ou du prix en mode manuel. Bouton equivalent :
  pave de l onglet Caisse
- `,` : separateur des centimes. Bouton equivalent : touche `,` du pave
- `*` : mode manuel : multiplie le prix tape. Mode scan : multiplie la derniere ligne du
  panier. Bouton equivalent : touche `x` du pave
- `+` : ajoute l article au panier (mode manuel), ou valide une quantite. Bouton
  equivalent : `+ Ajouter l article`, dans le pave a droite des zeros, visible en mode
  manuel seulement
- `Retour arriere` : efface le dernier chiffre. Bouton equivalent : touche fleche du
  pave
- `Entree` : ferme la sommation, puis valide la vente. Bouton equivalent : le grand
  bouton sous le pave, qui porte le geste suivant (voir juste en dessous)
- `Echap` : **Defait le dernier geste**, un a la fois. Bouton equivalent : aucun

Aucune autre touche n est prise en compte dans l ecran Caisse.

#### Le grand bouton sous le pave

Un seul bouton porte toute la vente au doigt. **Son libelle dit ce que le prochain appui
fera**, et il change tout seul :

- `Totaliser` : mode manuel, pendant la saisie des prix. Ferme la sommation, exactement
  comme la premiere `Entree`.
- `Montant exact` : pose le total dans `Somme recue du client`. C est l etat du bouton
  des que le panier porte un article et que rien n est tape.
- `Encaisser (Entree)` : la vente part. C est l etat du bouton des qu une somme recue
  est posee ou tapee.
- `Valider la quantite` : pendant une multiplication en mode scan.

Il est grise quand l appui ne menerait a rien : panier vide, ou somme recue inferieure
au total.

*Pourquoi un seul bouton :* au doigt, deux boutons voisins pour deux gestes qui se
suivent toujours dans le meme ordre font une erreur de plus a commettre. Le bouton et la
touche `Entree` passent par le meme code : ils ne peuvent pas divergir.

#### Ce que defait la touche Echap

`Echap` revient en arriere d un cran, le geste le plus recent d abord. Elle ne jette
jamais la vente entiere.

- **Curseur sur `Somme recue du client`, mode manuel** : le champ `Prix de l article`
  revient, le panier est intact, la sommation reprend ou elle s etait arretee
- **Multiplication en cours (`1500 x 3`)** : la multiplication est retiree, le prix
  reste
- **Un prix en cours de frappe** : le prix est efface, le panier reste
- **Une somme recue tapee** : le montant est efface, le panier reste
- **Rien dans les champs** : la derniere ligne du panier s en va, et ainsi de suite
- **Panier vide** : `Rien a defaire`

Pour jeter la vente en cours d un seul geste : bouton `Annuler la vente`. Pour annuler
une vente **deja encaissee** : chapitre 5.8.

*Pourquoi :* une frappe de trop sur `Echap` ne doit pas couter un panier de vingt
articles un samedi midi.

Deux touches du pave qui font gagner du temps :

- `00` : ajoute deux zeros d un coup.
- `C` : efface la somme recue sans toucher au panier.

*Pourquoi les chiffres tapes vite sont ignores :* LCOS mesure la cadence de frappe. Une
rafale trop rapide pour une main humaine est traitee comme un scan, jamais comme un
montant. C est ce qui empeche un code-barres d atterrir dans la somme recue.

### 5.6 Activer ou desactiver le ticket

**Emplacement rapide** : `LCOS --> Caisse --> sous les boutons de mode --> badge ticket`.
Un clic bascule, le code du gerant est demande s il n est pas deja deverrouille.

**Emplacement complet** : `LCOS --> Parametrage --> Commerce et ticket --> case Ticket active --> bouton Enregistrer`

Pour arreter la sortie du papier sans toucher au ticket lui-meme :
`LCOS --> Parametrage --> Impression des recus` (chapitre 4.3). La vente est alors
confirmee par un eclair d une seconde au centre de l ecran.

### 5.7 Fermer la session

**Emplacement** : `LCOS --> Session --> Fermer la session`

```
champ Code --> bouton Fermer la session
```

Accepte le code du caissier en poste **ou** celui du gerant.

*Pourquoi le gerant aussi :* un caissier parti sans fermer sa session ne doit pas
bloquer la caisse ni la cloture du jour.

---

### 5.8 Annuler une vente deja encaissee

**Emplacement** : `LCOS --> icone a trois barres --> Session --> panneau Ventes enregistrees`

Possible seulement si le gerant a ouvert l option (chapitre 4.3 bis). Sinon le panneau
se lit, mais rien ne s y annule.

Le panneau liste **toutes les ventes de la periode**, la plus recente en haut. Quatre
lignes sont visibles : descends dans la liste pour voir les precedentes.

```
clic sur la ligne de la vente --> touche Suppr (ou clic sur la croix x)
--> taper le code --> bouton Annuler cette vente
```

- Le code attendu est celui **du caissier qui a fait la vente**, ou celui du **gerant**.
- La question affiche le numero du ticket, le montant, le caissier et l heure : relis-la
  avant de valider.
- Mauvais code : `code du caissier de la vente ou du gerant attendu`. La vente reste.
- Bouton `Renoncer` : la demande se referme, rien n est touche.

Apres l annulation : le stock des articles est rendu, les totaux de la session et du
rapport Z redescendent, et la ligne part dans
`LCOS --> Parametrage --> Rapports --> Ventes annulees` avec le nom de qui l a annulee.

*Pourquoi un code a chaque fois :* la croix et la touche `Suppr` sont des gestes rapides.
Une vente, elle, est de l argent deja compte. Le code est a la fois la confirmation et
la signature.

---

## 6. Rapports et remise a zero

**Emplacement** : `LCOS --> Parametrage --> Rapports`

L onglet exige le code du gerant : `bouton Saisir le code du gerant --> Parametrage --> Deverrouiller --> revenir sur Rapports`.

### 6.1 Consulter les ventes des caissiers

- **Compteurs**
  - Emplacement : `Rapports` (haut de page)
  - Contenu : ventes en memoire, `Total vente` (ce que les ventes de la periode ont
    rapporte), grand total, nombre de remises a zero, dernier numero de ticket
- **Journal**
  - Emplacement : `Rapports --> Ventes des caissiers`
  - Contenu : une ligne par vente. Liste `Tous les caissiers` pour filtrer, bouton
    `Rafraichir` pour recharger
- **Ticket d une vente**
  - Emplacement : `Rapports --> Ventes des caissiers --> ligne --> bouton Ticket`
  - Contenu : reaffiche le ticket, reimprimable
- **Totaux**
  - Emplacement : `Rapports --> Totaux par caissier`
  - Contenu : nombre de ventes et montant encaisse par caissier

Les colonnes du journal : `Ticket`, `Date`, `Caissier`, **`Articles`** (les noms des
articles vendus, avec `x3` quand la ligne en portait trois), `Qte` (le nombre d articles
du ticket), `Total`.

### 6.2 Repartir a zero (rapport Z)

**Emplacement** : `LCOS --> Parametrage --> Rapports --> Rapport Z et remise a zero`

```
fermer la session --> bouton Previsualiser le rapport Z --> case Je confirme l effacement --> bouton Editer le rapport Z et repartir a zero
```

1. `Previsualiser le rapport Z` n efface rien : il montre exactement ce qui sera imprime.
2. La cloture imprime le rapport, puis **efface la memoire des ventes**.

**Ce qui est efface :** les ventes et leurs lignes de detail.

**Ce qui n est jamais efface :**

- les grands totaux (cumul depuis l installation),
- le compteur de remises a zero,
- le numero de ticket, qui continue sa numerotation,
- toute la programmation : rayons, produits, caissiers, compte gerant, reglages.

La cloture est refusee si une session est encore ouverte, ou s il n y a aucune vente.

### 6.2 bis Lire les ventes annulees

**Emplacement** : `LCOS --> Parametrage --> Rapports --> Ventes annulees`

Une ligne par vente retiree du journal depuis la derniere remise a zero :

- `Ticket` : le numero annule, celui qui manque dans la suite
- `Encaissee le` : quand la vente avait ete faite
- `Caissier` : qui l avait encaissee
- `Annulee le` : quand elle a ete annulee
- `Par` : le compte dont le code a servi a l annuler
- `Total` : le montant sorti de la caisse

La remise a zero emporte ce tableau avec les ventes de la periode : il decrit la periode
en cours, pas toute la vie de la caisse. Imprime ou note le rapport Z avant de repartir
a zero si ces lignes doivent etre gardees.

### 6.2 ter Exporter le detail des ventes et vider la memoire

**Emplacement** : `LCOS --> Parametrage --> Rapports --> Vider la memoire des ventes`

```
fermer la session --> case Je confirme l export du detail puis l effacement des ventes
--> bouton Exporter le detail et vider la memoire
```

Un seul geste pour deux choses : le detail part dans un fichier, puis la periode repart
a zero exactement comme le rapport Z du chapitre 6.2.

**Le fichier** : `lcos_ventes_AAAAMMJJ-HHMM.csv`, dans `Documents\LCOS` (le meme dossier
que les sauvegardes). Le chemin complet s affiche sous le bouton apres l operation.
Double clic sur le fichier : il s ouvre dans Excel ou LibreOffice Calc.

**Ce qu il contient**, dans l ordre :

- **Entete** : nom du commerce, numero du rapport Z, date d edition, premiere et
  derniere vente, plage des numeros de ticket, nombre de ventes, nombre d articles,
  total vente, grand total
- `Detail des ventes` : **Une ligne par article vendu** : ticket, date et heure,
  caissier, article, code-barres, rayon, quantite, prix unitaire, total de la ligne,
  total du ticket, somme recue, rendu
- `Totaux par caissier` : nombre de ventes et montant par caissier
- `Totaux par rayon` : quantite et montant par rayon

Une ligne par article, c est ce qui permet de trier, filtrer et faire un tableau croise
dans le tableur : c est le seul endroit ou le detail des ventes survit a la remise a
zero.

**Ce qui est efface, ce qui reste** : exactement comme le rapport Z. Les ventes et leur
detail partent ; les grands totaux, le compteur de remises a zero, le numero de ticket
et toute la programmation restent. Un rapport Z est edite et archive au passage, donc il
reste relisible dans `Rapports Z archives`.

**Refus possibles** : une session de caisse encore ouverte, ou aucune vente en memoire.

*Pourquoi le fichier d abord :* LCOS ecrit le fichier **avant** d effacer. Si l ecriture
echoue (cle retiree, disque plein), rien n est efface. Pas de fichier, donc pas de perte.

### 6.3 Relire un ancien rapport Z

**Emplacement** : `LCOS --> Parametrage --> Rapports --> Rapports Z archives --> ligne --> bouton Relire`

Le texte imprime de chaque cloture est conserve : c est la trace lisible des periodes
deja effacees.

---

## 7. Douchette code-barres

**Emplacement** : `LCOS --> Parametrage --> Peripheriques`

Branche la douchette sur un port USB et scanne une etiquette : la detection est
automatique, aucun pilote a installer. Le badge passe a `detecte` et la signature du
lecteur est memorisee.

Si rien ne se passe : `bouton Calibrer la douchette` --> scanne une etiquette. En
calibration, toute frappe est acceptee et les mesures reelles du lecteur s affichent
(nombre de caracteres, cadence, suffixe). Cela montre si le lecteur emet bien, et a
quelle vitesse.

---

## 7 bis. Imprimante du ticket

**Emplacement** : `LCOS --> Parametrage --> Peripheriques --> panneau Imprimante du ticket`

L imprimante integree de la caisse est vue par Windows comme une imprimante ordinaire :
elle doit donc apparaitre dans `Parametres Windows --> Imprimantes et scanners` avant que
LCOS puisse s en servir.

```
bouton Detecter --> liste Imprimante retenue --> liste Voie d impression -->
cases ESC/POS --> bouton Enregistrer --> bouton Test d impression
```

- `Imprimante retenue` : L imprimante qui sort les tickets. Son etat est indique entre
  parentheses.
- `Voie d impression` : trois choix, detailles juste apres.
- **Badge en haut a droite** : nombre d imprimantes vues par Windows.

- `Fenetre d impression Windows`
  - Ce qu elle fait : ouvre la fenetre habituelle de Windows.
  - Quand la choisir : pour imprimer sur une autre imprimante ou verifier la mise en
    page.
- `Envoi direct : texte`
  - Ce qu elle fait : le ticket part sur le rouleau, sans aucune boite de dialogue.
  - Quand la choisir : imprimante ordinaire, ou pilote qui refuse l ESC/POS.
- `Envoi direct : ESC/POS thermique 58 mm`
  - Ce qu elle fait : parle le langage de la thermique : coupe du papier, tiroir-caisse,
    gras.
  - Quand la choisir : **C est le reglage a retenir pour l imprimante integree de la
    caisse.**

La voie ESC/POS ouvre trois cases, sans effet sur les deux autres voies :

- `Couper le papier en fin de ticket` : la lame coupe le ticket. A decocher si la
  machine n a pas de lame.
- `Ouvrir le tiroir-caisse` : impulsion sur le connecteur du tiroir a chaque ticket.
- `Entete et totaux en gras` : nom du commerce en gras et double hauteur, lignes `TOTAL`
  en gras.

*Pourquoi une voie a part :* la coupe du papier, le tiroir et le gras ne sont pas du
texte, ce sont des octets de commande. La fenetre d impression de Windows et l envoi
texte les avalent ; seul l envoi ESC/POS les transporte jusqu au materiel.

*Pourquoi la detection peut echouer :* LCOS interroge le service de spoule de Windows. Si
l imprimante integree n a pas de pilote installe, Windows ne la liste pas et LCOS ne peut
pas la voir. Installe d abord son pilote, allume-la et charge le rouleau, puis reviens
sur `Detecter`.

Le bouton `Test d impression` envoie une mire de 32 caracteres. Si la ligne de chiffres
tient sur une seule ligne, la largeur du ticket est correcte. Sinon :
`LCOS --> Parametrage --> Commerce et ticket --> Largeur (colonnes)`.

---

## 7 ter. Afficheur client (le petit ecran du client)

**Emplacement** : `LCOS --> Parametrage --> Peripheriques --> Afficheur client`

C est le petit ecran tourne vers le client, a l arriere de la caisse. Il n affiche
qu une somme a la fois : celle que le caissier a sous les yeux, mise a jour en direct.

**Ce n est pas un second ecran d ordinateur.** Il ne sait rendre que des caracteres, il
n a pas de carte graphique derriere lui, et Windows ne le montre donc **jamais** comme un
moniteur. Chercher un deuxieme ecran dans les reglages d affichage de Windows ne donnera
rien : ce n est pas la qu il se configure.

### Ce que le client voit, et quand

- **Chaque article scanne, ajoute ou retire** : le total a payer, mis a jour aussitot
- **Chaque chiffre de la somme recue du client** : ce montant en train de se taper, pour
  que le client verifie qu on a bien lu son billet
- **Somme recue effacee** : retour au total a payer
- **Apres l encaissement, s il y a de la monnaie a rendre** : **le rendu, tenu environ 8
  secondes**, puis retour a zero
- **Apres l encaissement, compte juste** : le total encaisse reste affiche jusqu a la
  vente suivante
- **Vente suivante commencee avant la fin des 8 secondes** : le nouveau total reprend la
  main aussitot : le client d apres ne lit pas la monnaie du precedent

### Ce qu il montre, et ce qu il ne montre pas

- Le total a payer, cale a droite, a chaque changement du panier.
- La somme recue du client des que le caissier la tape, chiffre par chiffre, dans les
  deux modes.
- Jamais le prix d un article en cours de frappe en mode manuel : c est la sommation qui
  reste devant le client tant que l article n est pas ajoute.
- Rien du tout si la case est decochee : LCOS n ouvre meme pas le port.

*Si l afficheur ne repond plus en pleine journee :* la vente continue. LCOS signale une
seule fois `Afficheur client muet`, passe le badge en defaut, et n essaie plus jusqu au
prochain `Enregistrer`. Un ecran arriere en panne ne doit pas arreter une caisse.

Le reglage de cet afficheur (branchement, port, vitesse, protocole) se fait une
fois a l installation : voir `recherches/guide_editeur_lcos.md`, chapitre 2.

---

## 8. Depannage

- **L onglet Caisse affiche `Aucune session de caisse ouverte`**
  - Cause : pas de session
  - Action : `LCOS --> Session --> Ouvrir une session`
- **Un scan repond `code inconnu`**
  - Cause : produit absent ou code different
  - Action : `LCOS --> Parametrage --> Produits` --> cree le produit en scannant l
    etiquette dans `Code-barres`
- **Le grand bouton sous le pave reste grise**
  - Cause : panier vide ou somme recue inferieure au total
  - Action : ajoute un article, ou complete la somme recue
- **`code incorrect` au deverrouillage**
  - Cause : code faux
  - Action : 5 essais puis blocage 60 s : attends, puis ressaie
- **`trop d essais`**
  - Cause : blocage anti-devinette
  - Action : attends le delai indique
- **La cloture Z est refusee**
  - Cause : session ouverte, ou aucune vente
  - Action : `LCOS --> Session --> Fermer la session`, puis recommence
- **Le reglage de l heure est refuse**
  - Cause : droits administrateur manquants
  - Action : `LCOS --> Parametrage --> Date et heure --> bouton Autoriser LCOS en administrateur`,
    sinon `bouton Ouvrir le panneau Windows`
- **Le contenu d un onglet ne change pas**
  - Cause : corrige depuis la 1.3.0
  - Action : mets a jour avec le setup `LCOS_1.4.0_x64-setup.exe`
- **Je ne trouve plus les onglets Rapports, Peripheriques, Diagnostic**
  - Cause : ils sont passes dans le Parametrage depuis la 1.4.0
  - Action :
    `icone a trois barres --> Parametrage --> code du gerant --> sous-onglet voulu`
- **Je ne trouve plus la barre d onglets**
  - Cause : remplacee par l icone a trois barres de l entete
  - Action : `en haut a droite, a cote de la version`
- **Le total affiche encore une virgule en FCFA**
  - Cause : decimales restees activees
  - Action :
    `LCOS --> Parametrage --> Devise et decimales --> Desactivees --> Enregistrer`
- **La touche `,` a disparu de la calculatrice**
  - Cause : normal : decimales desactivees
  - Action : aucune action. Les montants se tapent en entiers
- **Une feuille sort encore apres chaque vente**
  - Cause : impression des recus activee
  - Action : `LCOS --> Parametrage --> Impression des recus --> decocher --> Appliquer`
- **Plus aucun ticket ne sort, mais le rapport Z sort**
  - Cause : normal depuis la 1.4.0
  - Action : le reglage ne touche que la caisse. Reactive-le dans `Impression des recus`
- **Le papier ne se coupe pas, le tiroir ne s ouvre pas**
  - Cause : voie d impression sans ESC/POS
  - Action :
    `LCOS --> Parametrage --> Peripheriques --> Voie d impression --> Envoi direct : ESC/POS thermique 58 mm --> Enregistrer`
- **`envoi ESC/POS refuse par Windows`**
  - Cause : imprimante absente, eteinte, ou pilote qui refuse les donnees brutes
  - Action : verifie l imprimante dans Windows, sinon reviens a `Envoi direct : texte`
- **Impossible de creer un produit : stock refuse**
  - Cause : le champ `Stock` attend `NON` ou un nombre
  - Action : remets `NON` dans `Stock`
- **Je dois reprogrammer une seconde caisse a l identique**
  - Cause : rien a retaper
  - Action :
    `LCOS --> Parametrage --> Export et import --> Exporter le parametrage seul` sur une
    cle USB, puis importe le fichier sur l autre caisse
- **L import repond `N vente(s) en memoire`**
  - Cause : le parametrage seul effacerait des comptes rattaches a ces ventes
  - Action : edite un rapport Z, puis recommence l import
- **Le code du gerant ne marche plus apres un import**
  - Cause : normal : c est desormais celui du fichier importe
  - Action : saisis le code de la caisse qui a produit le fichier
- **Aucun fichier dans la liste d import**
  - Cause : mauvais dossier, ou aucun `.json` dedans
  - Action : corrige `Dossier`, puis `bouton Relire le dossier`
- **`version d essai : 1 caissier(s) au maximum`**
  - Cause : plafond de la semaine d essai
  - Action : insere une licence (`Parametrage --> Licence`), le plafond tombe aussitot
- **`version d essai : l export demande une licence`**
  - Cause : L export et l import sont fermes pendant l essai
  - Action : idem ci-dessus
- **La liste `Fichier du dossier` est vide a l import**
  - Cause : aucun fichier `.json` dans le dossier d export
  - Action : corrige le `Dossier` du panneau Exporter puis `Relire le dossier`, ou tape
    le chemin complet du fichier
- **En mode scan, `Entree` n encaisse pas tout de suite**
  - Cause : C est voulu depuis la 1.8.0 : la premiere `Entree` pose le total dans la
    somme recue
  - Action : touche `Entree` une seconde fois
- **Le fichier .csv affiche les accents en caracteres abimes**
  - Cause : le tableur a ouvert le fichier dans le mauvais jeu de caracteres
  - Action : ouvre-le par `Donnees --> A partir d un fichier texte`, jeu de caracteres
    `UTF-8`, separateur `point-virgule`
- **Le fichier .csv met tout dans une seule colonne**
  - Cause : le tableur attend la virgule comme separateur
  - Action : meme chemin que ci-dessus, separateur `point-virgule`
- **`Echap` ne vide plus la vente**
  - Cause : C est voulu depuis la 1.7.0 : elle defait un geste a la fois
  - Action : pour jeter la vente entiere : bouton `Annuler la vente`
- **Aucune croix sur les lignes des ventes, `Suppr` ne fait rien**
  - Cause : L option d annulation est fermee
  - Action : `LCOS --> Parametrage --> Commerce et ticket --> Annulation des ventes`
- **`annulation des ventes desactivee`**
  - Cause : meme cause, vue du noyau
  - Action : idem ci-dessus
- **`code du caissier de la vente ou du gerant attendu`**
  - Cause : code d un autre caissier, ou code faux
  - Action : le code du caissier qui a fait **cette** vente, ou celui du gerant
- **Un numero de ticket manque dans la suite**
  - Cause : cette vente a ete annulee
  - Action : `LCOS --> Parametrage --> Rapports --> Ventes annulees`
- **La touche `*` ne multiplie pas**
  - Cause : elle n agit qu en mode manuel, sur le champ du prix
  - Action : `bouton Mode manuel`, tape un prix, puis `*`
- **`Caisse fermee : licence requise`**
  - Cause : les sept jours d essai sont passes
  - Action : colle une licence dans le champ, la caisse rouvre aussitot. Les donnees
    sont intactes
- **La licence est refusee**
  - Cause : cle incomplete ou recopiee a la main
  - Action : recolle la cle entiere, de `LCOS1.` jusqu au dernier caractere
- **L editeur demande l empreinte du poste**
  - Cause : elle identifie cette caisse
  - Action : `LCOS --> Parametrage --> Licence`, ligne `Empreinte de ce poste`
- **Le badge `essai : N j` gene**
  - Cause : il disparait sous licence perpetuelle
  - Action : rien a faire : il n est la que pendant l essai ou avec une licence a terme
- **Aucune imprimante detectee**
  - Cause : pilote absent, imprimante eteinte
  - Action : installe le pilote dans Windows --> allume l imprimante -->
    `LCOS --> Parametrage --> Peripheriques --> bouton Detecter`
- **Le ticket ne sort pas tout seul**
  - Cause : voie d impression sur `Fenetre d impression Windows`
  - Action :
    `LCOS --> Parametrage --> Peripheriques --> Voie d impression --> Enregistrer`
- **Le bouton `+` n ajoute rien en mode manuel**
  - Cause : champ prix vide ou prix a zero
  - Action : tape un prix superieur a `0` avant la touche `+`
- **Un rayon refuse d etre supprime**
  - Cause : il contient des produits
  - Action : deplace ou supprime ses produits, ou decoche `Rayon actif`
- **Un caissier refuse d etre supprime**
  - Cause : il a des ventes enregistrees
  - Action : `bouton Modifier --> decocher Compte actif`
- **Le ticket sort coupe a droite**
  - Cause : largeur mal reglee
  - Action : `LCOS --> Parametrage --> Commerce et ticket --> Largeur (colonnes)` = `32`

---

## 9. Sauvegarde

Toutes les donnees tiennent dans trois fichiers :

```
C:\Users\<utilisateur>\AppData\Roaming\org.lcos.caisse\lcos.db
C:\Users\<utilisateur>\AppData\Roaming\org.lcos.caisse\lcos.db-wal
C:\Users\<utilisateur>\AppData\Roaming\org.lcos.caisse\lcos.db-shm
```

**Ferme LCOS**, puis copie les trois fichiers sur une cle USB. Pour restaurer : remets
les trois fichiers au meme endroit, LCOS fermee.

Sauvegarde sans fermer LCOS, et lisible a l oeil :
`LCOS --> Parametrage --> Export et import --> Exporter avec les ventes` (chapitre 4.9).
C est la methode a preferer au quotidien ; la copie des trois fichiers reste le filet de
secours quand la base elle-meme est en cause.

*Pourquoi les trois :* le fichier `-wal` contient les ecritures les plus recentes. Copier
`lcos.db` seul peut perdre les dernieres ventes.

Verification rapide de l etat de l installation : `LCOS --> Parametrage --> Diagnostic`. La carte
`Base de donnees` doit afficher `operationnelle` et indiquer le chemin exact du fichier
en service.
