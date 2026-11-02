# Tiroir-caisse — caisse P70L

## Ce qu'il est, et pourquoi on ne le détectera jamais

Le tiroir n'a **aucune électronique**. Son petit câble, qui ressemble à un
RJ45 en plus étroit, est un **RJ11/RJ12** et il va à **l'imprimante de
ticket**, sur son port marqué `DK` ou `Drawer` — jamais à l'ordinateur.

Il ne s'annonce donc à rien : ni USB, ni PnP, ni port COM. **« Relever le
matériel » ne le montrera pas**, et ce n'est pas un défaut. C'est l'imprimante
qui l'ouvre, en envoyant du courant dans la bobine du pêne quand elle reçoit
l'impulsion ESC/POS `ESC p`.

## Vérifié sur le matériel, le 2026-11-02

`1B 70 00 19 FA` (`ESC p 0 25 250`, impulsion sur la broche 2) envoyé sur
`LPT1:` par `cmd /c copy /b` : **le tiroir s'ouvre**. La chaîne matérielle est
donc bonne, imprimante comprise.

## Ce qui bloquait, côté réglages

| Réglage | Enregistré le 24/10 | À corriger en |
|---|---|---|
| Mode d'impression | `escpos` | — ✓ |
| **Imprimante retenue** | **Microsoft XPS Document Writer** | **LCOS Ticket LPT1** |
| **Ouvrir le tiroir-caisse** | **non** | **oui** |

L'imprimante retenue était **virtuelle** : les tickets partaient dans un
fichier XPS, et aucune impulsion ne pouvait atteindre le tiroir. Seul le mode
`escpos` transporte les octets bruts ; `navigateur` et `systeme` n'envoient
que du texte et n'ouvriront jamais le tiroir.

## Modifications du code source, le 2026-11-02

Avant, l'impulsion ne partait **que dans le flux d'un ticket imprimé**. Sans
reçu — impression désactivée, ou ticket inactif — le tiroir restait fermé,
alors qu'une vente avait bien été encaissée.

| Fichier | Ce qui change |
|---|---|
| `domaine/escpos.rs` | `impulsion_tiroir()` expose l'impulsion seule. `encoder()` est inchangé, son test aussi |
| `commandes/impression.rs` | Commande `ouvrir_tiroir_caisse` : envoie l'impulsion sans rien imprimer. Rend `false` sans agir si le tiroir n'est pas demandé, si aucune imprimante n'est retenue, ou si le mode ne porte pas les octets bruts |
| `lib.rs` | La commande est enregistrée |
| `vues/caisse.js` | À l'encaissement, `ticketSort()` dit si du papier sort. Si oui, l'impulsion voyage déjà dans le ticket ; sinon elle part seule. L'échec est avalé : un tiroir muet n'arrête jamais une vente |

Pas de double impulsion : les deux voies s'excluent.

## L'imprimante virtuelle — cause racine et correctifs

**Comment le XPS Document Writer a été retenu.** `memoriser_detection()`
choisissait l'imprimante **par défaut de Windows**, sans regarder son port.
Or un poste neuf sort avec `Microsoft Print to PDF` ou le XPS en défaut. LCOS
a donc retenu une imprimante virtuelle tout seul, le 24 octobre, et rien ne
pouvait le signaler : le spouleur avale les octets sans erreur, qu'il y ait du
matériel au bout ou non.

| Fichier | Correctif |
|---|---|
| `commandes/impression.rs`, `memoriser_detection` | Le matériel passe d'abord ; le défaut de Windows ne départage que les imprimantes branchées entre elles. Et une imprimante **virtuelle déjà retenue est remplacée** dès qu'une imprimante branchée existe |
| `commandes/impression.rs`, `enregistrer_reglages_impression` | Refuse d'enregistrer une imprimante virtuelle en mode `systeme` ou `escpos`, via `exiger_imprimante_materielle()`. Le message nomme les imprimantes branchées disponibles. Le mode `navigateur` n'est pas concerné. Spouleur illisible ou imprimante absente de l'énumération : le doute profite au réglage, l'enregistrement passe |

**Conséquence** : le réglage se répare de lui-même. Au premier relevé du
matériel après recompilation, `LCOS Ticket LPT1` remplacera le XPS, et il ne
sera plus possible de retenir une virtuelle par mégarde.

Le test `distingue_les_ports_materiels_des_ports_logiciels` couvrait déjà la
reconnaissance des ports ; `PORTS_MATERIELS` vaut `COM`, `LPT`, `USB`, `IP_`,
`WSD-`.
