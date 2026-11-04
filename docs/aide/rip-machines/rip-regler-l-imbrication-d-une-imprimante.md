---
titre: Régler l'imbrication d'une imprimante
description: Choisissez la stratégie, le chargement, les espaces, la justification, la bande de purge et les marges des planches automatiques d'une imprimante.
sous_rubrique: rip-machines
position: 43
mots_cles: imbrication, stratégie, un par un, groupé, économie de support, chargement, justification, purge, nesttrimbox
---

Au lieu de sortir une à une, les impressions en attente d'une même imprimante peuvent partir ensemble sur une planche. Cet article décrit les réglages d'une imprimante, pour l'administrateur.

## Avant de commencer

Il vous faut un compte administrateur. Le survol de chaque ligne de la fiche donne l'aide.

## Étapes

1. Ouvrez **Administration…**, vue **Système**, section **Machines**, fiche de l'imprimante.
2. Choisissez l'**Imbrication** (stratégie) :
   - **Un par un** (défaut) : chaque impression sort seule, rien n'est imbriqué ;
   - **Groupé dans l'ordre** : côte à côte, par priorité (échéance la plus proche d'abord), sans rotation ni réordonnancement ;
   - **Économie de support** : toute la planche est recalculée à chaque arrivée, les plus hauts d'abord, avec le quart de tour quand il raccourcit la planche.
3. Choisissez le **Chargement** : **Libre (laize du rouleau)** (la planche prend la laize du rouleau chargé, sinon celle de la machine) ou **Strict (une planche par format)** (les travaux de formats différents ne partagent pas une planche).
4. Réglez l'**Espace entre copies (mm)** (5 par défaut) : blanc entre deux éléments côte à côte, et entre rangées ; **Espace entre rangées (mm)** le remplace entre rangées (vide : le même).
5. Choisissez la **Justification** : **Gauche**, **Centre** ou **Droite**. Centré ou à droite, la planche fait toute la laize. Réglez au besoin le **Décalage d'origine X (mm)** (0 à 1000) : blanc à gauche de la planche, avant tout le reste.
6. Choisissez la **Bande de purge** : **Aucune**, **À gauche**, **À droite** ou **Des deux côtés**, et la **Largeur de purge (mm)** (1 à 100, 10 par défaut).
7. Réglez **Marge haute (mm)** et **Marge basse (mm)** (0 à 200), et cochez **Contour de toute la planche (couche NestTrimBox)** si la planche part sur une table de découpe.
8. Cochez **Retourner un lé sur deux** pour un raccord de teinte entre lés voisins.
9. Réglez les déclencheurs de départ (voir [Régler les déclencheurs de départ d'une planche](rip-regler-les-declencheurs-de-depart-d-une-planche.md)), puis cliquez **Enregistrer**.

## Bon à savoir

- Format d'un travail : la laize de son support au catalogue, sinon celle pour laquelle sa mosaïque est faite ; sans format connu, le rouleau.
- Bande de purge : elle court le long de toute la planche, au bord choisi (à gauche, après le décalage d'origine), en quatre bandes égales cyan, magenta, jaune, noir, pour dégorger chaque tête ; l'espace entre copies la sépare des éléments.
- **Retourner un lé sur deux** : les lés pairs d'une mosaïque (2, 4…) sont posés à l'envers (demi-tour, même place).
- Le contour NestTrimBox entoure les éléments posés (sans le décalage ni la purge), dans la couche `NestTrimBox`, coupée en dernier, jamais imprimée.
- Ce qui s'imbrique ensemble et la planche en préparation : voir [Suivre une planche en préparation dans la file](../rip-travaux/rip-suivre-une-planche-dans-la-file.md).

## Voir aussi

- [Régler les déclencheurs de départ d'une planche](rip-regler-les-declencheurs-de-depart-d-une-planche.md)
- [Suivre une planche en préparation dans la file](../rip-travaux/rip-suivre-une-planche-dans-la-file.md)
- [Préparer une planche pour une table de découpe](rip-preparer-une-planche-pour-une-table-de-decoupe.md)
