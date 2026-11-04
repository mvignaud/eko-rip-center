---
titre: Séparer les pages et refuser les fichiers douteux à l'arrivée
description: Créez un travail par page d'un PDF, refusez les fichiers dont une police manque, et comprenez l'ordre des rendus simultanés.
sous_rubrique: rip-travaux
position: 65
mots_cles: une page par travail, refuser si une police manque, rendus simultanés, dossier surveillé, refuses
---

Deux cases du panneau d'envoi, gardées dans le réglage, servent surtout aux dossiers surveillés. Cet article s'adresse à l'administrateur.

## Avant de commencer

Vous avez un réglage enregistré (voir [Déposer un fichier dans un dossier surveillé](rip-deposer-un-fichier-dans-un-dossier-surveille.md)).

## Étapes

1. Cliquez **Importer** et réglez le travail.
2. Pour un travail par page, cochez **Une page par travail** : un PDF de plusieurs pages devient un travail par page, nommés « fichier.pdf (p. 1/5) », « fichier.pdf (p. 2/5) »… avec les mêmes réglages.
3. Pour refuser les fichiers douteux, cochez **Refuser si une police manque** : le fichier est refusé dès qu'une police n'est pas incorporée, avec la liste des polices.
4. Cliquez **Enregistrer…** pour garder ces cases dans le réglage.
5. Pour un dossier surveillé, retrouvez les fichiers refusés dans `refuses/`, avec leur fichier `.refus.txt`.

## Bon à savoir

- **Une page par travail** est sans effet sur une planche imbriquée.
- Les 14 polices de base du PDF (Helvetica, Times, Courier, Symbol, ZapfDingbats) ne comptent pas : le rendu les remplace par des polices de mêmes dimensions.
- Sans la case **Refuser si une police manque**, le fichier passe avec une alerte (voir [Lire les alertes d'un travail](rip-lire-les-alertes-d-un-travail.md)).
- **Plusieurs rendus à la fois** : le serveur rend plusieurs travaux en même temps (un par cœur, 4 au plus, réglable par `EKO_RIP_RENDUS`), montrés dans **Système** > **Rendus simultanés**.
- Les arrivées des dossiers surveillés et de l'ERP partent en file dans leur ordre de dépôt, même quand un petit fichier finit son rendu avant un gros arrivé plus tôt.
- Un fichier envoyé à la main depuis l'écran passe devant les arrivées automatiques qui attendent leur rendu, et part dès qu'il est prêt.

## Voir aussi

- [Déposer un fichier dans un dossier surveillé](rip-deposer-un-fichier-dans-un-dossier-surveille.md)
- [Régler le dossier automatique](rip-regler-le-dossier-automatique.md)
- [Lire les alertes d'un travail](rip-lire-les-alertes-d-un-travail.md)
