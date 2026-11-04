---
titre: Comprendre les rôles sur un site
description: Propriétaire, administrateur, opérateur, lecture : ce que chaque rôle peut faire sur un site du cloud.
sous_rubrique: rip-cloud-compte
position: 24
mots_cles: rôles, propriétaire, administrateur, opérateur, lecture, droits
---

Votre rôle sur un site décide de ce que vous pouvez faire depuis le portail et depuis les apps.
La colonne **Rôle** de **Mes sites** le rappelle pour chaque site.

## Avant de commencer

- Le compte qui rattache un serveur en devient **propriétaire**.
- Le partage d'un site avec d'autres comptes (collègues, prestataires) n'est pas encore proposé : pour l'instant, le propriétaire est le compte qui a rattaché le serveur.

## Étapes

1. Ouvrez **Mes sites**.
2. Lisez la colonne **Rôle** : **Propriétaire**, **Administrateur**, **Opérateur** ou **Lecture**.
3. Reportez-vous au tableau.

   | Rôle | Dans une app connectée au cloud | Sur le portail |
   |---|---|---|
   | Propriétaire | Administrateur du RIP | Retire le site, restaure les sauvegardes, reçoit les alertes |
   | Administrateur | Administrateur du RIP | Restaure les sauvegardes, reçoit les alertes |
   | Opérateur | Opérateur du RIP | Voit le site |
   | Lecture | Opérateur en lecture seule : aucune écriture | Voit le site |

   ![Liste Mes sites avec la colonne Rôle](../images/cloud-mes-sites.png)

## Bon à savoir

- Seul le **propriétaire** peut retirer un site depuis le portail. Voir [Détacher un serveur RIP du cloud](rip-cloud-detacher-un-serveur.md).
- Le propriétaire et les administrateurs reçoivent les alertes de site hors ligne et peuvent restaurer une sauvegarde.
- Un compte en **Lecture** qui tente une modification voit le message « Votre rôle sur ce site (cloud) est en lecture seule. »
- Ces rôles ne concernent que le cloud. Les comptes du serveur RIP ont leurs propres droits : voir [Gérer les comptes du serveur](../rip-administration/rip-gerer-les-comptes.md).

## Voir aussi

- [Connecter une app au cloud](../rip-cloud-premiers-pas/rip-cloud-connecter-une-app.md)
- [Consulter et gérer son compte](rip-cloud-gerer-son-compte.md)
- [Restaurer une sauvegarde de la configuration](../rip-cloud-sauvegardes/rip-cloud-restaurer-une-sauvegarde.md)
