---
titre: Comprendre les plaques d'encre et les noms de tons directs
description: Sachez comment le RIP sépare un fichier en plaques, quels noms donnent du blanc ou de l'argent et ce que deviennent les tons de découpe.
sous_rubrique: rip-couleur
position: 40
mots_cles: plaques, white, silver, metallic, blanc, argent, tons directs, none, noms reconnus
---

Le RIP rend une plaque par encre. Cet article liste les noms que le RIP reconnaît, pour préparer un fichier sans mauvaise surprise.

## Avant de commencer

Vous préparez un fichier dans votre logiciel de mise en page (par exemple Illustrator) ou vous contrôlez un travail rendu.

## Étapes

1. Nommez les tons directs du fichier selon ce tableau.

   | Plaque | Nom du ton direct |
   |---|---|
   | Cyan, Magenta, Jaune, Noir | Les quatre encres de base du fichier. |
   | **Blanc** | `White`, `Blanc` ou `WH` (Epson conseille `White`). |
   | **Argent métallisé** | `Silver`, `Metallic`, `Argent` ou `MS` (Epson conseille `Metallic`). |
   | Autre ton direct | Une plaque par ton du PDF ; à convertir en CMJN ou à déclarer dans la table des tons directs. |

2. Envoyez le fichier, puis cliquez le travail et ouvrez **Couches** dans l'inspecteur : une vignette par plaque.
3. Sur l'aperçu d'une plaque, le noir est l'encre et le blanc l'absence d'encre.
4. Pour poser le blanc ou l'argent sous la couleur ou dessus (vitrophanie), utilisez **Blanc ou argent** dans la section Blanc du travail.

## Bon à savoir

- Les tons de découpe (`CutContour`…) ne sont jamais imprimés et n'effacent pas l'encre sous eux, même sans surimpression dans le fichier : au rendu, ils deviennent le colorant `None` de la norme PDF. Il n'y a donc pas de liseré blanc le long de la découpe.
- Les tons directs qui ont leur encre (`White`, `Silver`…) ne sont jamais convertis en CMJN.
- La mise en file est refusée, avec la raison, pour un autre ton direct (le convertir en CMJN, le renommer, ou l'ajouter à la table des tons directs), ou pour du blanc et de l'argent sur la même page.
- Les couleurs métallisées `MT-*` du nuancier Epson ne sont pas encore converties.
- Pour générer le blanc sans le dessiner, voir [Poser du blanc automatiquement](rip-poser-du-blanc-automatique.md).

## Voir aussi

- [Déclarer les tons directs de l'atelier](rip-declarer-les-tons-directs.md)
- [Poser du blanc automatiquement](rip-poser-du-blanc-automatique.md)
- [Comprendre le pilote direct de l'Epson](../rip-machines/rip-comprendre-le-pilote-direct-de-l-epson.md)
