---
titre: Mettre un travail en coupe
description: Tournez la coupe, décalez-la vers l'extérieur, ou répétez et mettez à l'échelle une découpe seule sans refaire le rendu.
sous_rubrique: rip-travaux
position: 57
mots_cles: mise en coupe, rotation de la coupe, décalage, miroir, échelle, copies, écart, origine
---

Le panneau **Mise en coupe** tourne le travail pour la découpeuse, pas le fichier, sans nouveau rendu. Cet article s'adresse aux opérateurs.

## Avant de commencer

Le travail a des tracés de découpe. Rien ne part tant que vous n'avez pas enregistré.

## Étapes

1. Cliquez le travail, puis ouvrez le panneau **Mise en coupe** de l'inspecteur.
2. Réglez la **Rotation de la coupe** par quart de tour, cadre de repérage compris : tournez le support d'autant. L'aperçu de coupe porte une flèche grise du sens d'avance (le haut de la page entre le premier).
3. Réglez **Décaler la coupe vers l'extérieur (mm)**, de 0 à 5 mm : la pièce grandit d'autant (un trou se resserre).
4. Pour un travail imprimé dont la découpe courbe, voir [Compenser le cintrage](rip-compenser-le-cintrage.md).
5. En **découpe seule** uniquement, réglez aussi : **Miroir**, **Échelle X / Y (%)** (**Proportionnelle** ou non), **Copies X / Y** avec **Écart X / Y (mm)** (100 par sens, 1000 en tout) et **Départ depuis l'origine de la machine**.
6. Cliquez **Enregistrer la mise en coupe** : les fichiers de coupe sont refaits.

## Bon à savoir

- Miroir, échelle et copies ne sont proposés qu'en découpe seule : avec une impression, la coupe n'a pas d'encre à suivre.
- Le décalage ne remplace pas l'étalonnage imprimante-découpeuse (**Décalage X / Y** de la machine) : voir [Corriger une découpe décalée par rapport à l'impression](../rip-depannage/rip-decoupe-decalee.md).
- Une coupe tournée ou en miroir ne se superpose plus à l'impression dans la visionneuse.

## Voir aussi

- [Compenser le cintrage](rip-compenser-le-cintrage.md)
- [Ordonner les couches de découpe](rip-ordonner-les-couches-de-decoupe.md)
- [Corriger une découpe décalée par rapport à l'impression](../rip-depannage/rip-decoupe-decalee.md)
