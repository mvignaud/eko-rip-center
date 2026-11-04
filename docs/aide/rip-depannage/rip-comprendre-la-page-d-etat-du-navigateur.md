---
titre: Comprendre pourquoi le navigateur n'affiche qu'une page d'état
description: Un navigateur ouvert sur l'adresse du serveur n'affiche qu'une page d'état : l'interface complète est réservée aux clients et au Cloud.
sous_rubrique: rip-depannage
position: 21
mots_cles: navigateur, page d'état, client_requis, interface, administration, cloud, réservé au client
---

Si vous ouvrez l'adresse du serveur dans un navigateur, vous ne voyez qu'une page d'état. C'est voulu : l'interface complète ne vit que dans le client E-KO RIP Center.

## Avant de commencer

Vous connaissez l'adresse du serveur et le client est installé sur votre poste (Mac, Linux ou Windows).

## Étapes

1. Constatez la page d'état : version du serveur, machines et adresse à saisir dans le client.

   ![Page d'état du serveur affichée dans un navigateur](../images/serveur-page-etat.png)
2. Pour travailler, ouvrez le client : voir [Se connecter au serveur depuis un poste](../rip-premiers-pas/rip-se-connecter-au-serveur.md).
3. Pour les réglages, ouvrez le menu **Administration…** du client.
4. De l'extérieur de l'atelier, ouvrez l'adresse du site sur le Cloud dans un navigateur : voir [Accéder à son serveur à distance](../rip-cloud-premiers-pas/rip-cloud-acceder-a-distance.md).

## Bon à savoir

- Une requête d'un navigateur à l'API reçoit « Réservé au client E-KO RIP Center » (`client_requis`).
- Cette page est aussi un moyen de vérifier qu'un serveur répond sur le réseau : si elle s'affiche, le service tourne et le port 8640 est joignable.
- Pour le développement seulement, `EKO_RIP_ACCES_NAVIGATEUR=1` ouvre l'interface complète dans un navigateur : ne l'activez pas en production.

## Voir aussi

- [Le client ne trouve pas le serveur](rip-client-ne-trouve-pas-le-serveur.md)
- [Comprendre les pièces du RIP](../rip-premiers-pas/rip-comprendre-les-pieces-du-rip.md)
- [Modifier les réglages avancés du serveur](../rip-administration/rip-modifier-les-reglages-avances-du-serveur.md)
