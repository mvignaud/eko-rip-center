---
titre: Restaurer une sauvegarde de la configuration
description: Remplacez la configuration du serveur RIP par celle d'une sauvegarde en ligne, ou vers un autre site avec l'offre Pro.
sous_rubrique: rip-cloud-sauvegardes
position: 2
mots_cles: restaurer, sauvegarde, configuration, autre site
---
Restaurer remet la configuration du serveur RIP dans l'état d'une sauvegarde.
Le propriétaire et les administrateurs du site peuvent le faire depuis le portail.

## Avant de commencer

- Une restauration remplace la configuration actuelle du serveur.
- En cas de doute, téléchargez d'abord une sauvegarde de l'état actuel.

## Étapes

1. Ouvrez **Mes sites**, puis cliquez sur **Sauvegardes** sous le nom du site.
2. Repérez la version voulue : date, motif et taille.
3. Au besoin, cliquez d'abord sur **Télécharger** pour en garder une copie.
4. Sur la ligne de la version, cochez **Remplacer les réglages actuels**.
5. Cliquez sur **Restaurer**, puis confirmez dans la fenêtre qui s'ouvre.

   ![Page Sauvegardes d'un site : trois versions avec leur date, leur motif, leur taille, le lien Télécharger, la case Remplacer les réglages actuels et le bouton Restaurer](../images/cloud-sauvegardes.png)

## Bon à savoir

- Avec l'offre **Pro**, une sauvegarde peut aussi être restaurée vers un **autre site** du même propriétaire.
- Voir [Restaurer une sauvegarde vers un autre site](rip-cloud-restaurer-vers-un-autre-site.md) pour cet usage.
- La sauvegarde ne contient ni les travaux ni les comptes : après une restauration, les comptes du serveur ne changent pas.
- Le serveur reçoit l'ordre par la connexion avec la plate-forme, vérifie qu'il en vient bien (ordre signé, valable quelques minutes, utilisable une seule fois), télécharge l'archive et l'applique. Un ordre sans signature valide est refusé avec le message « Ordre du cloud refusé : signature absente, fausse ou périmée. »
- Un e-mail « Sauvegarde restaurée » prévient le propriétaire et les administrateurs.
- Le serveur doit être en ligne : voir [Résoudre un site hors ligne](../rip-cloud-depannage/rip-cloud-site-hors-ligne.md).

## Voir aussi

- [Comprendre les sauvegardes en ligne](rip-cloud-comprendre-les-sauvegardes.md)
- [Alertes de place et d'échec](rip-cloud-alertes-de-sauvegarde.md)
