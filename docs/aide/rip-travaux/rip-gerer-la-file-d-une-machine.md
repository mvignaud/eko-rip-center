---
titre: Gérer la file d'une machine
description: Passez une sortie en tête, suspendez-la, annulez-la ou refaites-la, et comprenez pourquoi une sortie attend.
sous_rubrique: rip-travaux
position: 3
mots_cles: file, sortie, suspendre, reprendre, annuler, refaire, historique, attente
---

Chaque machine passe une sortie à la fois, dans l'ordre de sa file. Cet article montre comment réordonner la file et lire les raisons d'attente.

## Avant de commencer

Des sorties sont en file (voir [Suivre les travaux et lancer l'impression](rip-suivre-les-travaux.md)). Un compte **Opérateur** suffit.

## Étapes

1. Ouvrez la file de la machine : cliquez son nom dans la barre latérale, ou ouvrez la vue **File d'attente**.
2. Cliquez la sortie à traiter.
3. Pour changer l'ordre, cliquez **En tête**, **Monter** ou **Descendre**. Vous pouvez aussi glisser une sortie en attente à la place d'une autre de la même machine.
4. Pour la mettre de côté, cliquez **Suspendre** ; les suivantes passent devant elle. Cliquez **Reprendre** pour la libérer.
5. Pour l'abandonner, cliquez **Annuler**.
6. Pour la repasser, ouvrez l'**Historique** et cliquez **Refaire** : la même sortie repart sans refaire le rendu.
7. Pour nettoyer l'historique, cliquez **Retirer**.

## Bon à savoir

Une sortie qui ne peut pas partir affiche un badge rouge (le survol donne le détail) et laisse passer les suivantes :

- **Machine injoignable** : elle part dès que la machine répond ;
- **Fichier manquant** : relancez le travail ;
- **Trop large** : la page, repères compris, dépasse la laize de la machine ou du rouleau chargé ;
- **Autre support** : elle part d'elle-même quand le bon support est chargé ;
- **Impression pas finie** : une découpe attend la fin de l'impression de sa page ;
- **Découpeuse indisponible** : l'impression attend, pour ne pas gâcher d'encre ;
- **En imbrication** : l'impression attend sa planche ;
- **Page exclue** : la case **Imprimer cette page** est décochée.

Chaque impression affiche sa durée estimée. Pour l'ajuster, mesurez la cadence réelle et saisissez-la en **Cadence (m²/h)** de la machine. Une sortie coupée par un arrêt du serveur passe en *Échec* : **Refaire** la relance. L'**Archive** garde les découpes archivées, que **Restaurer** remet en file.

## Voir aussi

- [Créer une planche pour regrouper des impressions](rip-creer-une-planche.md)
- [La sortie vers la machine a échoué](../rip-depannage/rip-sortie-vers-la-machine-echouee.md)
- [Ajouter une machine au RIP](../rip-machines/rip-ajouter-une-machine.md)
- Page détaillée : https://e-ko.rip/documentation/utilisation/
