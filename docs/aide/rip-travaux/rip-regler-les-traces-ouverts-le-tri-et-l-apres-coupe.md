---
titre: Régler les tracés ouverts, le tri et l'après-coupe
description: Autorisez les tracés ouverts, choisissez l'ordre de coupe des tracés et décidez du sort d'une découpe finie (historique, archive ou suppression).
sous_rubrique: rip-travaux
position: 60
mots_cles: tracés ouverts, évents de bâche, tri des tracés, après coupe, archive, restaurer, couper dès l'arrivée
---

Trois réglages de la section **Découpe** et un réglage de machine décident comment les tracés sont coupés et ce que devient la découpe finie. Cet article s'adresse à l'administrateur.

## Avant de commencer

Il vous faut un compte administrateur. Le bloc **Découpe** est dans la vue **Système**.

## Étapes

1. Ouvrez **Administration…**, vue **Système**, bloc **Découpe**.
2. Cochez ou décochez **Autoriser les tracés ouverts (évents de bâche)** (coché par défaut). Décoché, un tracé ouvert n'est pas coupé, avec une alerte par page.
3. Cochez ou décochez **Trier les tracés** (coché par défaut) : intérieurs d'abord, puis le plus proche voisin. Décoché, les tracés sont coupés dans leur ordre de création.
4. Choisissez **Après coupe** : **ne rien faire (historique)** (par défaut), **archiver (restaurable)** ou **supprimer de la file**.
5. Cliquez **Enregistrer**.
6. Pour que chaque découpe arrive en attente, ouvrez **Système > Machines**, fiche de la découpeuse, et décochez **Couper dès l'arrivée** (coché par défaut). Chaque découpe arrive alors suspendue et part quand l'opérateur clique **Reprendre**.

## Bon à savoir

- Une découpe archivée sort de la file et va dans l'**Archive** de sa machine ; **Restaurer** la remet en file (suspendue sur une machine sans coupe à l'arrivée), **Retirer** l'efface.
- Cochez aussi **Couper aussi les tracés des calques non imprimables** dans ce bloc pour couper les tracés des calques non imprimables.
- Ces réglages valent pour les rendus suivants.

## Voir aussi

- [Régler les préfixes de découpe](rip-regler-les-prefixes-de-decoupe.md)
- [Gérer la file d'une machine](rip-gerer-la-file-d-une-machine.md)
- [Ordonner les couches de découpe](rip-ordonner-les-couches-de-decoupe.md)
