---
titre: Régler les déclencheurs de départ d'une planche
description: Décidez quand une planche imbriquée part : délai, absence d'arrivée, surface, longueur cible, pourcentage ou rangée pleine.
sous_rubrique: rip-machines
position: 44
mots_cles: déclencheurs, départ après, longueur cible, surface, rangée pleine, imprimer maintenant, planche suivante
---

Une planche imbriquée part quand son déclencheur est atteint. Cet article décrit chaque déclencheur, pour l'administrateur.

## Avant de commencer

L'imbrication est activée sur l'imprimante (voir [Régler l'imbrication d'une imprimante](rip-regler-l-imbrication-d-une-imprimante.md)).

## Étapes

1. Ouvrez la fiche de l'imprimante, section **Machines** de la vue **Système**.
2. Renseignez les déclencheurs voulus. Un champ vide = pas de déclencheur ; avec plusieurs, le premier atteint déclenche.

   | Déclencheur | Effet |
   |---|---|
   | **Départ après (min)** | Minutes depuis l'arrivée du **premier** travail de la planche. |
   | **Départ sans arrivée depuis (min)** | Minutes sans **nouveau** travail (depuis le dernier arrivé). |
   | **Départ à (m²)** | Surface des travaux posés, copies comprises. |
   | **Longueur cible (mm)** | Les travaux qui la feraient dépasser attendent la planche suivante (badge *attend la planche suivante*), et une planche pleine part d'elle-même. Un travail seul plus long que la cible part quand même, seul. |
   | **Départ à (% de la cible)** | La planche part quand elle atteint ce pourcentage de la longueur cible (demande une longueur cible). |
   | **Départ à la longueur (mm)** | La planche part dès qu'elle atteint cette longueur ; rien n'est reporté (le travail qui la franchit part avec elle). |
   | **Rangée pleine sortie seule** | Le travail qui ouvrirait une deuxième rangée attend, et la rangée pleine part seule. |

3. Cliquez **Enregistrer**.

## Bon à savoir

- Sans aucun déclencheur, la planche attend que vous cliquiez **Imprimer maintenant**.
- **Imprimer maintenant** est dans la planche en préparation de la vue **File d'attente**.

## Voir aussi

- [Régler l'imbrication d'une imprimante](rip-regler-l-imbrication-d-une-imprimante.md)
- [Suivre une planche en préparation dans la file](../rip-travaux/rip-suivre-une-planche-dans-la-file.md)
- [Créer une planche pour regrouper des impressions](../rip-travaux/rip-creer-une-planche.md)
