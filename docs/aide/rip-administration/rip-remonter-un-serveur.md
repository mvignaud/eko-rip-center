---
titre: Remonter un serveur à partir d'une sauvegarde
description: Réinstallez le serveur sur un nouveau PC et restaurez la configuration depuis le fichier de sauvegarde.
sous_rubrique: rip-administration
position: 4
mots_cles: restaurer, sauvegarde, nouveau serveur, remplacement, configuration, comptes
---

Quand vous changez de PC serveur, la sauvegarde de la configuration vous évite de tout régler à nouveau. Cet article s'adresse à l'administrateur.

## Avant de commencer

Vous avez le fichier de sauvegarde de la configuration (voir [Sauvegarder la configuration](rip-sauvegarder-la-configuration.md)) et un PC prêt pour le serveur.

## Étapes

1. Installez le serveur sur le nouveau PC (voir [Installer le serveur RIP](../rip-premiers-pas/rip-installer-le-serveur.md)).
2. Créez le compte administrateur (voir [Créer les comptes](rip-creer-les-comptes.md)).
3. Dans le client, connectez-vous à ce nouveau serveur.
4. Ouvrez **Administration… › Système › Sauvegarde de la configuration**, puis cliquez **Restaurer…**.
5. Choisissez le fichier de sauvegarde.
6. Recréez les comptes opérateurs.
7. Vérifiez la liste des machines dans **Système › Machines**.

## Bon à savoir

- Tout est vérifié avant d'écrire : un fichier abîmé ne change rien.
- Les travaux ne sont pas dans la sauvegarde : ils se refont depuis leurs fichiers sources.
- Réservez la même adresse fixe au nouveau serveur, ou changez de serveur dans chaque client avec **Changer de serveur…**.

## Voir aussi

- [Sauvegarder la configuration du serveur](rip-sauvegarder-la-configuration.md)
- [Installer le serveur RIP](../rip-premiers-pas/rip-installer-le-serveur.md)
- [Se connecter au serveur depuis un poste](../rip-premiers-pas/rip-se-connecter-au-serveur.md)
- Page détaillée : https://e-ko.rip/documentation/administration/
