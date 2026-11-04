---
titre: L'impression attend « Autre support » ou affiche « Trop large »
description: Corrigez une sortie qui attend parce que le support chargé n'est pas celui du travail ou que la page dépasse la laize du rouleau.
sous_rubrique: rip-depannage
position: 41
mots_cles: autre support, trop large, rouleau chargé, laize, support chargé, epson
---

Une sortie reste en file avec un badge rouge « Autre support » ou « Trop large ». Cet article donne la marche à suivre, pour les opérateurs.

## Avant de commencer

Vous voyez la sortie en file dans la vue **Production**. Pour lire le média, l'Epson doit avoir une adresse SNMP (voir [Lire le média chargé et la consommation réelle de l'Epson](../rip-couleur/rip-lire-le-media-charge-dans-l-epson.md)).

## Étapes

1. Survolez le badge rouge de la sortie pour lire le détail.
2. Pour **Autre support** : chargez dans l'Epson le support du travail. La sortie part d'elle-même.
3. Si le support est déjà le bon mais mal reconnu, ouvrez **Système** > **Média chargé dans l'Epson** et choisissez le support à la main dans la liste (le choix prime jusqu'au retour à **— lu sur l'imprimante —**).
4. Pour **Trop large** : la page, repères compris, dépasse la laize de la machine ou du rouleau chargé. Chargez un rouleau plus large, ou réduisez la page (voir [Recadrer, redimensionner ou tourner une page](../rip-travaux/rip-recadrer-redimensionner-ou-tourner-une-page.md)).
5. Si besoin, cliquez **Reprendre** ou **Refaire** sur la sortie.

## Bon à savoir

- Un n° de support inconnu du catalogue est signalé : déclarez-le (voir [Régler un support et ses modes d'impression](../rip-couleur/rip-regler-un-support-et-ses-modes.md)).
- Support chargé inconnu : rien n'est bloqué.
- Une page peut être tournée automatiquement quand elle dépasse la laize mais tient tournée de 90° (case **Tourner seul si le visuel dépasse la laize**).

## Voir aussi

- [Comprendre les raisons d'attente d'une sortie](../rip-travaux/rip-comprendre-les-raisons-d-attente-d-une-sortie.md)
- [Gérer la file d'une machine](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
- [Brancher l'Epson SureColor SC-S80600](../rip-machines/rip-brancher-epson-sc-s80600.md)
