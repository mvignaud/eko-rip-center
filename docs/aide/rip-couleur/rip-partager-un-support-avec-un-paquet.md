---
titre: Partager un support avec un paquet support
description: Exportez un support, ses modes et les réglages qui s'en servent, puis importez-le sur un autre serveur sans doublon.
sous_rubrique: rip-couleur
position: 52
mots_cles: paquet support, exporter, importer, support, modes, identifiants, autre serveur
---

Un paquet support transporte un support d'un serveur à l'autre. Cet article s'adresse à l'administrateur.

## Avant de commencer

Vous êtes administrateur des deux serveurs. Le serveur qui reçoit est au moins aussi à jour que celui qui exporte.

## Étapes

1. Sur le serveur d'origine, ouvrez **Système**, section **Supports de l'Epson**.
2. Cliquez **Exporter** sur la ligne du support : un fichier est téléchargé, avec ce support, ses modes et les réglages qui s'en servent, sans profil ICC.
3. Sur l'autre serveur, ouvrez la même section et cliquez **Importer un paquet support**.
4. Choisissez le fichier.
5. Ajoutez le profil ICC du support sur le second serveur (voir [Choisir les profils de couleur d'un travail](rip-choisir-les-profils-de-couleur.md)).

## Bon à savoir

- Les identifiants sont gardés, et un réimport ne fait pas de doublon.
- Un paquet fait par une version plus récente est refusé : mettez d'abord ce serveur à jour (voir [Mettre à jour le serveur](../rip-administration/rip-mettre-a-jour-le-serveur.md)).
- Les prix restent dans **Coûts d'encre et de support**, reliés par le nom.

## Voir aussi

- [Régler un support et ses modes d'impression](rip-regler-un-support-et-ses-modes.md)
- [Régler les coûts d'encre et de support](rip-regler-les-couts-d-encre-et-de-support.md)
- [Mettre à jour le serveur](../rip-administration/rip-mettre-a-jour-le-serveur.md)
