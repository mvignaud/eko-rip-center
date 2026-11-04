---
titre: Comprendre le rendu par bandes d'un très grand format
description: Pourquoi un très grand format est rendu par bandes puis recollé, combien de place il demande et pourquoi il est plus long.
sous_rubrique: rip-depannage
position: 22
mots_cles: très grand format, bandes, bâche, bigtiff, rendu long, disque, 3 go, mosaïque
---

Un rendu très long n'est pas une panne : le serveur découpe lui-même les très grandes pages en bandes. Cet article explique ce mécanisme et la place disque à prévoir.

## Avant de commencer

Vous avez un travail de très grand format (par exemple une bâche) dont le rendu est plus long que d'habitude.

## Étapes

1. Repérez le cas : une page dont le rendu dépasserait 3 Go, par exemple une bâche de 1,6 × 3 m à 720 dpi.
2. Laissez faire : le serveur la rend par bandes horizontales, puis recolle ses plaques. Rien n'est à régler.
3. Vérifiez la place libre dans la vue **Système** du client (ligne **Espace libre**) avant de lancer.
4. Surveillez la progression dans la file : le travail reste **en cours** plus longtemps.

## Bon à savoir

- Une plaque recollée de plus de 3,5 Go est écrite en BigTIFF.
- Une plaque pèse jusqu'à largeur × hauteur en points, soit près de 4 Go par encre pour cette bâche, avant compression : prévoyez la place.
- Si le rendu dépasse le délai maximal (30 minutes par défaut), relevez `EKO_RIP_DELAI_S` : voir [Modifier les réglages avancés du serveur](../rip-administration/rip-modifier-les-reglages-avances-du-serveur.md).
- Sous 5 Go libres, le serveur refuse un travail qui ne tiendrait pas : voir [Résoudre un rendu en échec ou un fichier refusé](rip-un-rendu-echoue.md).
- Plus de cœurs et de mémoire raccourcissent ces rendus : voir [Choisir le poste serveur](../rip-premiers-pas/rip-choisir-le-poste-serveur.md).

## Voir aussi

- [Résoudre un rendu en échec ou un fichier refusé](rip-un-rendu-echoue.md)
- [Choisir le poste serveur](../rip-premiers-pas/rip-choisir-le-poste-serveur.md)
- [Modifier les réglages avancés du serveur](../rip-administration/rip-modifier-les-reglages-avances-du-serveur.md)
