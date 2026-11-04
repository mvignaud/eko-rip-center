---
titre: Ordonner les couches de découpe
description: Réglez l'ordre de coupe, activez ou non chaque couche et choisissez son réglage de coupe : mi-chair, perforation, rainage et coupe à travers.
sous_rubrique: rip-travaux
position: 54
mots_cles: couches de découpe, ordre de coupe, réglage de coupe, rainage, perforation, v-cut, une couche à la fois
---

Chaque ton de découpe du fichier devient une couche, coupée dans l'ordre que vous choisissez. Cet article s'adresse aux opérateurs d'une découpeuse.

## Avant de commencer

Le travail a des tracés de découpe (voir [Régler les préfixes de découpe](rip-regler-les-prefixes-de-decoupe.md)). La découpeuse a au moins un réglage de coupe (voir [Régler la coupe du Kona](../rip-machines/rip-regler-la-coupe-du-kona.md)).

## Étapes

1. Cliquez le travail, puis ouvrez le panneau **Couches de découpe** de l'inspecteur : une couche par nom de ton, avec son nombre de tracés.
2. Réglez l'ordre avec ↑ et ↓. **L'ordre des couches est l'ordre de coupe** : mi-chair avant coupe à travers.
3. Cochez ou décochez la case d'une couche pour la couper ou non.
4. Choisissez son **Réglage de coupe**. Par défaut, **— même nom, sinon défaut —** : le réglage de la découpeuse qui porte le nom de la couche s'applique seul, sinon le premier de la machine. Vous pouvez en imposer un autre.
5. Cliquez **Enregistrer les couches** : les fichiers de coupe sont refaits. **Annuler** revient aux couches enregistrées.

## Bon à savoir

- Ordre de départ : rainage (`Creasing`, `Score`), perforation et V-cut d'abord, la coupe en dernier, quand la matière tient encore.
- Rien n'est gardé sans **Enregistrer les couches**. Les couches enregistrées restent au redémarrage et au rendu suivant.
- Un travail d'avant les couches garde son ancien réglage de coupe : il va à la couche du même nom ou à la seule couche ; sinon le panneau le signale, à choisir couche par couche.
- Dans une couche, un contour contenu dans un autre (le trou d'un « O », une fenêtre) est découpé avant lui, pour que la pièce ne bouge pas ; à profondeur égale, la lame va au contour le plus proche.
- Sur la machine, **Une couche à la fois** (Système > Machines) : pour une découpeuse qui ne change ni d'outil ni de réglage en cours de coupe, chaque couche devient une sortie, la suivante retenue jusqu'à **Reprendre**.
- Une couche sans réglage sur la découpeuse choisie la rend indisponible : l'impression attend.

## Voir aussi

- [Régler la coupe du Kona](../rip-machines/rip-regler-la-coupe-du-kona.md)
- [Mettre un travail en coupe](rip-mettre-en-coupe-un-travail.md)
- [Choisir la découpeuse d'un travail](rip-choisir-la-decoupeuse-d-un-travail.md)
