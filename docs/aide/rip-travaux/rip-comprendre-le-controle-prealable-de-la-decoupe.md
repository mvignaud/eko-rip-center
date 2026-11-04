---
titre: Comprendre le contrôle préalable de la découpe
description: Lisez les alertes de découpe du rendu (ton RVB, tracé avec fond, tracé au bord, filet mal aligné…) et corrigez le fichier avant de couper.
sous_rubrique: rip-travaux
position: 56
mots_cles: contrôle préalable, alertes de découpe, tracé, filet, rvb, bord de page, calque, tracés ouverts
---

Au rendu, le RIP vérifie les tracés de découpe et signale ce qui risque de mal couper. Cet article liste chaque alerte et sa correction.

## Avant de commencer

Le travail a été rendu et affiche le badge ⚠ (voir [Lire les alertes d'un travail](rip-lire-les-alertes-d-un-travail.md)).

## Étapes

1. Cliquez le travail et lisez les alertes de découpe.
2. Trouvez-les dans le tableau et corrigez le fichier d'origine.

   | Alerte | Correction |
   |---|---|
   | Ton de découpe défini en RVB | Le définir en CMJN, en ton direct. |
   | Tracé de découpe avec un fond | Un tracé de découpe est un filet seul : retirer le fond. |
   | Tracé qui touche le bord de la page | Le rentrer dans la page. |
   | Tracé recouvert par l'image | Le mettre sur le calque du dessus. |
   | Contour vectorisé (filet devenu surface : il serait coupé deux fois) | Garder le filet, ne pas vectoriser le contour. |
   | Filet aligné à l'intérieur ou à l'extérieur du tracé | L'aligner au centre. |
   | Tracé qui sort du plan de travail | Ce qui dépasse n'est pas coupé : le ramener dans le plan. |
   | Découpeuse choisie sans aucun ton de découpe reconnu | Les tons du fichier sont listés : renommer le ton ou ajouter son préfixe (voir [Régler les préfixes de découpe](rip-regler-les-prefixes-de-decoupe.md)). |
   | Tracé sur un calque non imprimable | Il n'est pas coupé, sauf avec **Couper aussi les tracés des calques non imprimables** (Système > Découpe). |
   | Tracés ouverts non coupés | Autoriser les tracés ouverts, si les tracés ouverts ne sont pas autorisés (voir [Régler les tracés ouverts, le tri et l'après-coupe](rip-regler-les-traces-ouverts-le-tri-et-l-apres-coupe.md)). |

3. Renvoyez le fichier corrigé, ou acceptez l'alerte : le travail part quand même.

## Bon à savoir

- Ces alertes s'ajoutent à celles de l'envoi (polices, résolution).
- Une page qui porte des repères de découpe (repérage Kona, cadre…) sans aucun tracé de coupe reçoit l'alerte : *Des repères de découpe sont imprimés, mais la page n'a aucun tracé de coupe : encre et support pour rien.*
- Un ton absent de la table des tons directs a sa propre alerte (voir [Déclarer les tons directs de l'atelier](../rip-couleur/rip-declarer-les-tons-directs.md)).

## Voir aussi

- [Régler les préfixes de découpe](rip-regler-les-prefixes-de-decoupe.md)
- [Lire les alertes d'un travail](rip-lire-les-alertes-d-un-travail.md)
- [Détourer automatiquement une image](rip-detourer-automatiquement-une-image.md)
