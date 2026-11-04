---
titre: Se connecter au cloud depuis le client Mac ou Linux
description: Accédez à votre serveur depuis l'extérieur de l'atelier avec le bouton Se connecter au cloud des clients Mac et Linux.
sous_rubrique: rip-premiers-pas
position: 71
mots_cles: cloud, client, mac, linux, à distance, plate-forme, notifications, déconnexion
---

Le cloud permet de joindre le serveur de l'atelier depuis l'extérieur, sans VPN. Les clients Mac et Linux ouvrent votre compte de la plate-forme dans le navigateur, puis listent vos sites.

## Avant de commencer

Le serveur est déjà relié à la plate-forme (voir [Relier le serveur au Cloud](../rip-cloud-premiers-pas/rip-cloud-relier-un-serveur.md)) et vous avez un compte de la plate-forme. Comptez 2 minutes.

## Étapes

1. Sur l'écran de connexion, choisissez **Se connecter au cloud** (sur Linux : menu **Cloud › Se connecter au cloud**).
2. Le navigateur s'ouvre sur la plate-forme. Connectez-vous et autorisez l'application.
3. Revenez au client : il liste les sites de votre compte, avec votre rôle. Un site hors ligne est grisé.
4. Cliquez le site voulu. Aucun mot de passe du RIP n'est demandé.

## Bon à savoir

- Le client rouvre le dernier site au lancement suivant.
- **Se déconnecter du cloud** (bouton de l'écran des sites ou menu **Serveur** sur Mac ; sur Linux, écran des sites ou menu **Se déconnecter** sur un site cloud) révoque l'accès. Vous pouvez aussi déconnecter les apps depuis votre compte sur le portail (voir [Déconnecter une app du cloud](../rip-cloud-compte/rip-cloud-deconnecter-les-apps.md)).
- Sur Linux, si le navigateur ne s'ouvre pas, l'adresse d'autorisation s'affiche : copiez-la dans votre navigateur. Le client attend le retour pendant 5 minutes au plus ; **Annuler** abandonne l'opération.
- Sur Mac, le jeton d'accès est gardé dans le trousseau ; sur Linux, dans un fichier de configuration de votre dossier personnel réservé à votre utilisateur.
- **Administration…** n'est pas encore disponible à distance : utilisez-la depuis le réseau de l'atelier.
- Tant que le compte est connecté, même sur un serveur local, le client interroge la plate-forme chaque minute et affiche une notification système : site hors ligne, de retour en ligne, nouvelle app connectée. Un clic sur la notification ouvre le site concerné. macOS demande l'autorisation juste après la première connexion. Sur Linux, une alerte l'annonce à la première connexion, et la case **Notifications du cloud** du menu principal, activée par défaut, les coupe.
- Vous pouvez aussi saisir directement l'adresse du site, par exemple `https://atelier.platform.e-ko.rip`, puis vous connecter avec votre compte du serveur.
- Le client Windows n'a pas ce bouton : saisissez l'adresse du site.

## Voir aussi

- [Connecter une app Mac, Linux ou mobile au cloud](../rip-cloud-premiers-pas/rip-cloud-connecter-une-app.md)
- [Se connecter au serveur depuis un poste](rip-se-connecter-au-serveur.md)
- [Activer les notifications du cloud](../rip-cloud-compte/rip-cloud-activer-les-notifications.md)
