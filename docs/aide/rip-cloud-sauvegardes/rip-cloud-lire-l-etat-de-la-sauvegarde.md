---
titre: Vérifier que la sauvegarde en ligne fonctionne
description: Contrôlez la date de la dernière sauvegarde et son état (ok, offre, quota, erreur) côté portail et côté serveur.
sous_rubrique: rip-cloud-sauvegardes
position: 23
mots_cles: état, dernière sauvegarde, quota, erreur, vérifier, résumé hebdomadaire
---

Une sauvegarde utile est une sauvegarde récente.
Voici comment vérifier en quelques secondes que la vôtre l'est.

## Avant de commencer

- Le serveur doit être relié au cloud, avec une offre qui inclut les sauvegardes.
- Le propriétaire et les administrateurs du site reçoivent les alertes.

## Étapes

1. Ouvrez **Mes sites**, puis **Sauvegardes** sous le nom du site.
2. Regardez la première ligne de la liste : la date doit être récente. Chaque nuit, une sauvegarde part si la configuration a changé.
3. Sur le serveur, ouvrez **Système › Cloud E-KO RIP** pour vérifier que l'état est **Connecté : accessible à distance**. Voir [Lire l'état du cloud sur le serveur](../rip-cloud-premiers-pas/rip-cloud-lire-l-etat-du-cloud-sur-le-serveur.md).
4. Chaque lundi, lisez le résumé hebdomadaire : il indique l'état des sauvegardes de chaque site.

## Bon à savoir

- L'état de la sauvegarde côté serveur est l'un de : **ok**, **offre** (l'offre n'inclut pas les sauvegardes), **quota** (plus de place) ou **erreur** (avec un message).
- Si aucune sauvegarde de nuit n'est reçue depuis plus de deux jours, un e-mail prévient le propriétaire et les administrateurs. Voir [Comprendre les alertes de place et d'échec des sauvegardes](rip-cloud-alertes-de-sauvegarde.md).
- Les deux causes les plus fréquentes sont un site hors ligne et un quota plein. Voir [Résoudre un site hors ligne](../rip-cloud-depannage/rip-cloud-site-hors-ligne.md).

## Voir aussi

- [Comprendre les sauvegardes en ligne](rip-cloud-comprendre-les-sauvegardes.md)
- [Comprendre la durée de conservation et la place des sauvegardes](rip-cloud-duree-de-conservation.md)
- [Consulter la liste des e-mails de la plate-forme](../rip-cloud-compte/rip-cloud-comprendre-les-emails.md)
