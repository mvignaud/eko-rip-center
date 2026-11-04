---
titre: Régler les encres claires et l'encre totale d'un mode
description: Choisissez la bascule des encres claires (douce, équilibrée, franche) et la limite d'encre totale d'un mode d'impression de l'Epson.
sous_rubrique: rip-couleur
position: 44
mots_cles: bascule des claires, encre totale, douce, équilibrée, franche, mode, lc, lm, lk, limite d'encre
---

Deux champs facultatifs d'un mode règlent la séparation des encres claires sur l'Epson (pilote direct). Cet article s'adresse à l'administrateur.

## Avant de commencer

Il vous faut un compte administrateur et un support déclaré (voir [Régler un support et ses modes d'impression](rip-regler-un-support-et-ses-modes.md)).

## Étapes

1. Ouvrez **Administration…**, vue **Système**, section **Supports de l'Epson**, et le champ **Modes** du support.
2. Après les passes, ajoutez la bascule des claires et l'encre totale, dans n'importe quel ordre : `Affiche = 720 dpi / 8 / franche / 260 %`.
3. Choisissez la **Bascule des claires**, c'est-à-dire l'endroit où la foncée prend le relais de la claire (Lc, Lm, Lk) :

   | Valeur | Effet |
   |---|---|
   | **douce** | Claire pleine jusqu'à 30 % de densité, relais long jusqu'à 70 %, un peu de claire gardée sous la foncée. Le moins de grain : dégradés et peau vus de près. |
   | **équilibrée** (par défaut) | Claire seule jusqu'à 24 %, relais jusqu'à 54 %. |
   | **franche** | Claire limitée à 60 %, relais de 18 à 33 %. Moins d'encre et séchage plus rapide : affichage vu de loin. |

4. Saisissez l'**Encre totale** (`260 %`, de 100 à 400 %) : la somme maximale des 9 encres en un point (300 % sans elle).
5. Cliquez **Enregistrer**, puis faites un tirage d'essai.

## Bon à savoir

- Les valeurs de la bascule sont des valeurs de départ, à confirmer sur papier.
- Un troisième champ facultatif donne le volume des gouttes petite, moyenne et grosse pour l'encre estimée : `Autre = 360 dpi / 4 / 5-10-15 pL`. Sans lui, 5-10-15 pL, des valeurs de départ à étalonner.
- Pour trouver la limite d'encre, voir [Proposer une limite d'encre](rip-proposer-une-limite-d-encre.md).

## Voir aussi

- [Régler un support et ses modes d'impression](rip-regler-un-support-et-ses-modes.md)
- [Proposer une limite d'encre](rip-proposer-une-limite-d-encre.md)
- [Comprendre le pilote direct de l'Epson](../rip-machines/rip-comprendre-le-pilote-direct-de-l-epson.md)
