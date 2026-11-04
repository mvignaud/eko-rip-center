---
titre: Comprendre les événements notifiés par l'application mobile
description: Tableau des événements qui déclenchent une notification : alerte machine, encre, cartouche vide, machine injoignable, échecs.
sous_rubrique: rip-travaux
position: 75
mots_cles: notifications, événements, encre, injoignable, échec, seuil
---

Cet article liste ce qui déclenche une notification sur le téléphone, et à quel moment.

## Avant de commencer

Les notifications sont activées (voir [Activer et régler les notifications de l'application mobile](rip-activer-les-notifications-de-l-application-mobile.md)).

## Étapes

1. Repérez l'événement dans le tableau.
2. Lisez la condition de déclenchement.
3. Agissez sur place si besoin : changement de support, remplacement de cartouche, vérification du réseau de la machine.

| Événement | Quand |
|---|---|
| Alerte de la machine | Une nouvelle alerte apparaît (capot ouvert, support épuisé…). Les états ordinaires (prête, impression, chauffe, démarrage, veille) ne notifient pas. |
| Encre presque vide | Une encre passe à 5 % ou moins. |
| Cartouche vide | Une encre atteint 0 %. |
| Machine injoignable | Trois relevés de suite sans réponse, puis une notification quand elle répond de nouveau. |
| Sortie en échec | Une impression ou une découpe échoue. |
| Rendu en échec | Le rendu d'un travail échoue (une seule notification par travail). |
| Fin d'impression | Seulement si **Fins d'impression** est activé. |

## Bon à savoir

- L'alerte est affichée en français ; un message que le RIP ne connaît pas encore reste dans la langue de la machine.
- Un téléphone dont le compte est supprimé ne reçoit plus rien.
- Les notifications sont aussi gardées dans l'onglet **Alertes**.

## Voir aussi

- [Activer et régler les notifications de l'application mobile](rip-activer-les-notifications-de-l-application-mobile.md)
- [Consulter les alertes de l'application mobile](rip-consulter-les-alertes-de-l-application-mobile.md)
- [Un rendu échoue](../rip-depannage/rip-un-rendu-echoue.md)
