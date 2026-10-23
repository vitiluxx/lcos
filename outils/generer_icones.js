"use strict";

const zlib = require("zlib");
const fs = require("fs");
const path = require("path");

const DOSSIER_SORTIE = path.join(__dirname, "..", "src-tauri", "icons");
const SURECHANTILLONNAGE = 4;

const COULEUR_FOND = [31, 111, 92, 255];
const COULEUR_PAPIER = [255, 255, 255, 255];
const COULEUR_LIGNE = [31, 111, 92, 255];

function tableCrc() {
  const table = new Int32Array(256);

  for (let octet = 0; octet < 256; octet += 1) {
    let valeur = octet;

    for (let passe = 0; passe < 8; passe += 1) {
      valeur = valeur & 1 ? 0xedb88320 ^ (valeur >>> 1) : valeur >>> 1;
    }

    table[octet] = valeur;
  }

  return table;
}

const CRC = tableCrc();

function crc32(tampon) {
  let valeur = -1;

  for (let position = 0; position < tampon.length; position += 1) {
    valeur = CRC[(valeur ^ tampon[position]) & 0xff] ^ (valeur >>> 8);
  }

  return (valeur ^ -1) >>> 0;
}

function bloc(type, donnees) {
  const longueur = Buffer.alloc(4);
  longueur.writeUInt32BE(donnees.length, 0);

  const corps = Buffer.concat([Buffer.from(type, "ascii"), donnees]);
  const controle = Buffer.alloc(4);
  controle.writeUInt32BE(crc32(corps), 0);

  return Buffer.concat([longueur, corps, controle]);
}

function encoderPng(largeur, hauteur, pixels) {
  const entete = Buffer.alloc(13);
  entete.writeUInt32BE(largeur, 0);
  entete.writeUInt32BE(hauteur, 4);
  entete[8] = 8;
  entete[9] = 6;
  entete[10] = 0;
  entete[11] = 0;
  entete[12] = 0;

  const brut = Buffer.alloc(hauteur * (largeur * 4 + 1));

  for (let ligne = 0; ligne < hauteur; ligne += 1) {
    const depart = ligne * (largeur * 4 + 1);
    brut[depart] = 0;
    pixels.copy(brut, depart + 1, ligne * largeur * 4, (ligne + 1) * largeur * 4);
  }

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    bloc("IHDR", entete),
    bloc("IDAT", zlib.deflateSync(brut, { level: 9 })),
    bloc("IEND", Buffer.alloc(0))
  ]);
}

function dansRectangleArrondi(x, y, gauche, haut, droite, bas, rayon) {
  if (x < gauche || x > droite || y < haut || y > bas) {
    return false;
  }

  const centreX = Math.min(Math.max(x, gauche + rayon), droite - rayon);
  const centreY = Math.min(Math.max(y, haut + rayon), bas - rayon);
  const ecartX = x - centreX;
  const ecartY = y - centreY;

  return ecartX * ecartX + ecartY * ecartY <= rayon * rayon;
}

function dansTicket(x, y, taille) {
  const gauche = 0.28 * taille;
  const droite = 0.72 * taille;
  const haut = 0.2 * taille;
  const basMoyen = 0.72 * taille;

  if (x < gauche || x > droite || y < haut) {
    return false;
  }

  const periode = (droite - gauche) / 4;
  const amplitude = 0.04 * taille;
  const position = (x - gauche) % periode;
  const montee = Math.abs(position / periode - 0.5) * 2;
  const limiteBasse = basMoyen - amplitude + montee * amplitude * 2;

  return y <= limiteBasse;
}

function dansLignes(x, y, taille) {
  const gauche = 0.36 * taille;
  const droite = 0.64 * taille;
  const epaisseur = 0.045 * taille;
  const positions = [0.31, 0.42, 0.53].map((ratio) => ratio * taille);

  if (x < gauche || x > droite) {
    return false;
  }

  return positions.some((depart) => y >= depart && y <= depart + epaisseur);
}

function teindre(taille) {
  const grande = taille * SURECHANTILLONNAGE;
  const accumulateur = new Float64Array(taille * taille * 4);
  const rayon = 0.22 * grande;

  for (let ligne = 0; ligne < grande; ligne += 1) {
    for (let colonne = 0; colonne < grande; colonne += 1) {
      let couleur = [0, 0, 0, 0];

      if (dansRectangleArrondi(colonne + 0.5, ligne + 0.5, 0, 0, grande - 1, grande - 1, rayon)) {
        couleur = COULEUR_FOND;

        if (dansTicket(colonne + 0.5, ligne + 0.5, grande)) {
          couleur = dansLignes(colonne + 0.5, ligne + 0.5, grande)
            ? COULEUR_LIGNE
            : COULEUR_PAPIER;
        }
      }

      const indice =
        (Math.floor(ligne / SURECHANTILLONNAGE) * taille +
          Math.floor(colonne / SURECHANTILLONNAGE)) *
        4;

      for (let canal = 0; canal < 4; canal += 1) {
        accumulateur[indice + canal] += couleur[canal];
      }
    }
  }

  const diviseur = SURECHANTILLONNAGE * SURECHANTILLONNAGE;
  const pixels = Buffer.alloc(taille * taille * 4);

  for (let position = 0; position < pixels.length; position += 1) {
    pixels[position] = Math.round(accumulateur[position] / diviseur);
  }

  return pixels;
}

function construirePng(taille) {
  return encoderPng(taille, taille, teindre(taille));
}

function construireIco(tailles) {
  const images = tailles.map((taille) => ({ taille, donnees: construirePng(taille) }));
  const entete = Buffer.alloc(6);
  entete.writeUInt16LE(0, 0);
  entete.writeUInt16LE(1, 2);
  entete.writeUInt16LE(images.length, 4);

  const repertoire = Buffer.alloc(images.length * 16);
  let decalage = entete.length + repertoire.length;

  images.forEach((image, rang) => {
    const base = rang * 16;
    repertoire[base] = image.taille >= 256 ? 0 : image.taille;
    repertoire[base + 1] = image.taille >= 256 ? 0 : image.taille;
    repertoire[base + 2] = 0;
    repertoire[base + 3] = 0;
    repertoire.writeUInt16LE(1, base + 4);
    repertoire.writeUInt16LE(32, base + 6);
    repertoire.writeUInt32LE(image.donnees.length, base + 8);
    repertoire.writeUInt32LE(decalage, base + 12);
    decalage += image.donnees.length;
  });

  return Buffer.concat([entete, repertoire, ...images.map((image) => image.donnees)]);
}

function ecrire(nom, donnees) {
  const destination = path.join(DOSSIER_SORTIE, nom);
  fs.writeFileSync(destination, donnees);
  console.log(nom + " -> " + donnees.length + " octets");
}

fs.mkdirSync(DOSSIER_SORTIE, { recursive: true });

ecrire("32x32.png", construirePng(32));
ecrire("128x128.png", construirePng(128));
ecrire("128x128@2x.png", construirePng(256));
ecrire("icon.png", construirePng(512));
ecrire("icon.ico", construireIco([16, 24, 32, 48, 64, 128, 256]));
