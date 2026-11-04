---
titre: Comprendre la durée de conservation et la place des sauvegardes
description: Combien de versions sont gardées selon l'offre, ce que devient une sauvegarde identique et ce qui se passe quand la place manque.
sous_rubrique: rip-cloud-sauvegardes
position: 22
mots_cles: conservation, quota, stockage, rotation, historique, offre
---

Le nombre de versions gardées et la place disponible dépendent de votre offre.
Cet article explique les règles pour éviter les surprises.

## Avant de commencer

- Ouvrez **Mes sites**, puis **Sauvegardes** : l'en-tête rappelle l'offre et la place utilisée (par exemple « Offre Gratuit du propriétaire : dernière version · 6 Mo utilisés sur 1 Go »).
- Les sauvegardes ne contiennent ni les travaux ni les comptes.

## Étapes

1. Repérez votre offre dans le tableau.

   | Offre | Sauvegardes en ligne | Stockage |
   |---|---|---|
   | Gratuit | dernière version | 1 Go |
   | À la machine | 30 versions | 5 Go par machine |
   | Atelier | 90 jours d'historique | 25 Go |
   | Pro | 1 an d'historique, restauration vers un autre site | 100 Go |

2. Ouvrez **Mes sites › Sauvegardes** et comparez la place utilisée au stockage de l'offre.

   ![Page Sauvegardes d'un site avec l'offre, la place utilisée et la liste des versions](../images/cloud-sauvegardes.png)
3. Si la place est presque pleine, souscrivez une offre plus grande ou laissez la rotation libérer des versions. Voir [Changer d'offre, de paiement ou résilier](../rip-cloud-abonnement/rip-cloud-changer-ou-resilier.md).

## Bon à savoir

- La version la plus récente est toujours gardée.
- Le serveur envoie une sauvegarde chaque nuit (à une minute tirée au hasard entre 1 h et 5 h, heure du serveur), environ 2 minutes après une modification de configuration, et à la demande d'un administrateur. Une sauvegarde identique à la précédente n'est pas stockée deux fois ; celle de la nuit part quand même et confirme à la plate-forme que le serveur sauvegarde bien.
- Sur une offre sans sauvegarde en ligne, le serveur n'insiste pas jusqu'à ce qu'une offre l'autorise.
- Quand le quota est plein, la plate-forme prévient le propriétaire et refuse les nouvelles sauvegardes jusqu'à ce qu'il y ait de la place. Voir [Comprendre les alertes de place et d'échec des sauvegardes](rip-cloud-alertes-de-sauvegarde.md).
- Au retour à l'offre Gratuite, les versions existantes restent 90 jours, puis seule la dernière est gardée.
- Quand un site est retiré, ses sauvegardes sont supprimées 90 jours après la dernière.
- La taille maximale d'une archive envoyée est de 200 Mo.

## Voir aussi

- [Comprendre les sauvegardes en ligne](rip-cloud-comprendre-les-sauvegardes.md)
- [Choisir une offre](../rip-cloud-abonnement/rip-cloud-choisir-une-offre.md)
- [Télécharger une sauvegarde en ligne](rip-cloud-telecharger-une-sauvegarde.md)
