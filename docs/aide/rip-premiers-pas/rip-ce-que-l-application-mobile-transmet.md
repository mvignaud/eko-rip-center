---
titre: Comprendre ce que l'application mobile transmet
description: Sachez où vont vos identifiants et le jeton de notification, et ce que l'éditeur reçoit, quand vous utilisez l'application E-KO RIP.
sous_rubrique: rip-premiers-pas
position: 76
mots_cles: confidentialité, application mobile, données, identifiants, jeton, notifications
---

L'application E-KO RIP ne contient ni publicité ni mesure d'audience. Cet article résume ce qu'elle transmet et à qui.

## Avant de commencer

Rien à préparer : cet article est une référence.

## Étapes

1. Sans connexion par le cloud, l'application parle uniquement au serveur RIP de l'atelier : l'éditeur ne reçoit rien.
2. Avec le cloud, la plate-forme E-KO RIP Cloud connaît votre compte, le nom du téléphone et, si les notifications sont actives, son jeton de notification.
3. Vos identifiants restent dans l'espace sécurisé du téléphone et ne sont envoyés qu'au serveur choisi.
4. Pour les notifications, un jeton du téléphone est transmis au serveur RIP (ou à la plate-forme si vous vous y connectez). Les messages transitent ensuite par le service de notifications d'Expo, puis par Apple ou Google.
5. Pour retirer ces données du téléphone, touchez **Se déconnecter** dans l'onglet **Réglages**.

## Bon à savoir

- Se déconnecter supprime les identifiants du téléphone et retire le téléphone de la liste des notifications.
- Un téléphone dont le compte est supprimé ne reçoit plus rien.
- La politique complète est publiée sur https://e-ko.rip/app/confidentialite/.

## Voir aussi

- [Installer l'application mobile E-KO RIP](rip-installer-l-application-mobile.md)
- [Activer et régler les notifications de l'application mobile](../rip-travaux/rip-activer-les-notifications-de-l-application-mobile.md)
- [Se connecter au serveur depuis l'extérieur avec l'application mobile](rip-connecter-l-application-mobile-via-le-cloud.md)
