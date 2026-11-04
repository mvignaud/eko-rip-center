---
titre: Comprendre les machines comptées dans l'offre
description: Voyez comment le serveur déclare ses machines à la plate-forme et lisez le tableau Utilisation de la page Abonnement.
sous_rubrique: rip-cloud-abonnement
position: 25
mots_cles: machines comptées, inventaire, imprimante, découpeur, utilisation, quota
---

L'offre se paie par machine : la plate-forme compte celles que votre serveur RIP déclare.
Il n'y a rien à saisir à la main.

## Avant de commencer

- Le serveur doit être relié au cloud et à jour.
- Offre Gratuite : 1 site, 1 imprimante et 1 découpeur.

## Étapes

1. Ouvrez **Abonnement** sur le portail.
2. Dans la rubrique **Utilisation**, lisez la ligne **Sites**, **Machines** (imprimantes et découpeurs) et **Sauvegardes**.

   ![Page Abonnement avec la rubrique Utilisation : sites, machines et sauvegardes comptés, et le tableau par site](../images/cloud-abonnement.png)
3. Dans le tableau, repérez pour chaque site le nombre d'**Imprimantes** et de **Découpeurs**, et la colonne **Inventaire reçu**.
4. Si **Inventaire reçu** indique « jamais (serveur RIP à mettre à jour) », mettez à jour le serveur : il enverra alors son inventaire.

## Bon à savoir

- Le serveur déclare ses machines (identifiant, nom, modèle) à l'ouverture de la connexion, à chaque changement du catalogue de machines et toutes les 6 heures.
- Les imprimantes d'un côté, les traceurs et tables de découpe de l'autre.
- Une machine qui imprime compte comme imprimante, y compris la Roland VersaCAMM, qui découpe elle-même.
- La plate-forme répond au serveur avec l'offre du site, et indique si elle inclut les sauvegardes ou si le site dépasse son forfait.
- Si le parc dépasse l'offre, voir [Régulariser un parc qui dépasse l'offre](rip-cloud-parc-depasse-l-offre.md).
- Les offres payantes incluent la sauvegarde en ligne de la configuration. Voir [Comprendre les sauvegardes en ligne](../rip-cloud-sauvegardes/rip-cloud-comprendre-les-sauvegardes.md).

## Voir aussi

- [Choisir une offre](rip-cloud-choisir-une-offre.md)
- [Régulariser un parc qui dépasse l'offre](rip-cloud-parc-depasse-l-offre.md)
- [Changer d'offre, de paiement ou résilier](rip-cloud-changer-ou-resilier.md)
