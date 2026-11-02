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
