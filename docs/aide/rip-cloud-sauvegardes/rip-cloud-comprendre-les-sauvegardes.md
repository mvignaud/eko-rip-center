---
titre: Comprendre les sauvegardes en ligne
description: Ce que contiennent les sauvegardes envoyées à la plate-forme, quand elles partent et combien de temps elles sont gardées.
sous_rubrique: rip-cloud-sauvegardes
position: 1
mots_cles: sauvegardes, configuration, versions, quota
---
Le serveur RIP envoie à la plate-forme une sauvegarde de sa configuration.
Elle sert à retrouver vos réglages après un incident.

## Avant de commencer

- Le serveur doit être relié au cloud.
- La sauvegarde est la même archive que **Système › Sauvegarde** : machines, réglages, profils ICC.
- Elle ne contient **ni les travaux ni les comptes**. Elle est chiffrée et hébergée en France.

## Étapes

1. Ouvrez **Mes sites** sur le portail.
2. Cliquez sur **Sauvegardes** sous le nom du site.
3. Consultez les versions : date, motif (**Nocturne**, **Après modification**…) et taille. L'en-tête rappelle l'offre et la place utilisée.

   ![Page Sauvegardes d'un site avec l'offre, la place utilisée et la liste des versions](../images/cloud-sauvegardes.png)

## Bon à savoir

- Une sauvegarde part chaque nuit si la configuration a changé, environ 2 minutes après une modification, et à la demande d'un administrateur.
- Durée de conservation : dernière version (Gratuit), 30 versions (À la machine), 90 jours (Atelier), 1 an (Pro). La plus récente est toujours gardée.
- Si la sauvegarde est ancienne, vérifiez que le site est en ligne et que le quota n'est pas plein.

## Voir aussi

- [Restaurer une sauvegarde](rip-cloud-restaurer-une-sauvegarde.md)
- [Alertes de place et d'échec](rip-cloud-alertes-de-sauvegarde.md)
- [Choisir une offre](../rip-cloud-abonnement/rip-cloud-choisir-une-offre.md)
- [Documentation du cloud](https://e-ko.rip/documentation/cloud/)
