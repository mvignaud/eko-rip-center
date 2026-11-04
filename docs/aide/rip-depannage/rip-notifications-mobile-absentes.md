---
titre: Je ne reçois pas les notifications de l'application mobile
description: Que vérifier quand l'application E-KO RIP n'envoie aucune notification : autorisation, interrupteur, serveur, inscription du téléphone.
sous_rubrique: rip-depannage
position: 70
mots_cles: notifications, application mobile, absentes, autorisation, inscription, internet
---

Vous ne recevez ni alerte de machine ni notification d'essai. Suivez ces vérifications dans l'ordre.

## Avant de commencer

L'application est connectée au serveur.

## Étapes

1. Dans les réglages du téléphone, vérifiez que les notifications sont autorisées pour l'application.
2. Dans l'application, ouvrez **Réglages** et vérifiez que l'interrupteur **Notifications** est activé.
3. Touchez **Envoyer une notification d'essai**.
4. Vérifiez que le serveur RIP est à jour et qu'il a un accès à Internet : c'est lui qui envoie les notifications des machines.
5. Touchez **Se déconnecter**, puis reconnectez-vous : cela renouvelle l'inscription du téléphone.

## Bon à savoir

- Sur le serveur, si la « sécurité renforcée des push » est activée sur le projet de notifications, son jeton d'accès se place dans `EKO_RIP_EXPO_JETON` (fichier `/etc/default/eko-rip-serveur`). Le serveur doit pouvoir joindre `https://exp.host` en sortie.
- Les états ordinaires d'une machine (prête, impression, chauffe, démarrage, veille) ne notifient pas, et **Fins d'impression** est désactivé par défaut.
- Un téléphone reste inscrit au cloud tant que l'application est ouverte au moins une fois tous les 30 jours.
- Les événements ratés restent lisibles dans l'onglet **Alertes**.

## Voir aussi

- [Activer et régler les notifications de l'application mobile](../rip-travaux/rip-activer-les-notifications-de-l-application-mobile.md)
- [Comprendre les événements notifiés par l'application mobile](../rip-travaux/rip-comprendre-les-evenements-notifies.md)
- [Consulter les alertes de l'application mobile](../rip-travaux/rip-consulter-les-alertes-de-l-application-mobile.md)
