---
titre: Comprendre les raisons d'attente d'une sortie
description: Lisez le badge rouge d'une sortie qui ne part pas (machine injoignable, autre support, trop large, impression pas finie…) et agissez.
sous_rubrique: rip-travaux
position: 44
mots_cles: raison d'attente, badge rouge, injoignable, autre support, trop large, impression pas finie, imbrication, page exclue
---

Une sortie qui ne peut pas partir le dit par un badge rouge, et laisse passer les suivantes. Cet article est la référence de chaque raison et de ce qu'il faut faire.

## Avant de commencer

Une sortie est en file (voir [Gérer la file d'une machine](rip-gerer-la-file-d-une-machine.md)). Survolez le badge pour lire le détail. Une sortie suspendue par un opérateur reste simplement **Suspendue** : ce n'est pas une raison d'attente.

## Étapes

1. Repérez le badge rouge sur la ligne de la sortie dans **À imprimer**.
2. Cherchez sa raison dans le tableau ci-dessous.
3. Appliquez l'action indiquée.
4. Au besoin, cliquez **Refaire** ou **Reprendre** une fois la cause levée.

| Raison | Ce que cela veut dire | Que faire |
|---|---|---|
| **Machine injoignable** | La machine est débranchée ou absente du catalogue. | Rien : la sortie part dès que la machine répond. Vérifiez le câble, l'adresse et la fiche de la machine. |
| **Fichier manquant** | Le rendu ou le tracé est introuvable. | Relancez le travail (**Relancer**). |
| **Trop large** | La page, repères compris, dépasse la laize de la machine ou du rouleau chargé. | Réduisez la page, ou chargez un rouleau plus large. |
| **Autre support** | Le support chargé n'est pas celui du travail. | Chargez le bon support : la sortie part d'elle-même. Ou choisissez le support chargé à la main. |
| **Impression pas finie** | Une découpe attend que l'impression de sa page soit finie. | Attendez la fin de l'impression. |
| **Découpeuse indisponible** | Une impression attend que la découpeuse affectée soit utilisable, pour ne pas gâcher d'encre. | Rétablissez la découpeuse (voir [Choisir la découpeuse d'un travail](rip-choisir-la-decoupeuse-d-un-travail.md)). |
| **En imbrication** | L'impression attend sa planche. | Voir [Suivre une planche dans la file](rip-suivre-une-planche-dans-la-file.md). |
| **Page exclue** | La case **Imprimer cette page** est décochée. | Recochez-la. |
| **Attend le BAT** | L'original attend l'épreuve sur une autre machine. | Voir [Valider un BAT avec le client](rip-valider-un-bat.md). |

## Bon à savoir

- Une impression posée sur une planche passe **Imbriquée** (« sur » le nom de la planche) : elle ne part plus seule, la planche l'imprime. Elle ne se **Refait** pas tant que sa planche n'est pas imprimée (ce serait l'imprimer deux fois) ; **Retirer** reste possible.
- Une sortie coupée par un arrêt du serveur ne repart pas seule (une découpe ne se reprend pas au milieu) : elle passe en *Échec*, et **Refaire** la relance.
- Support chargé inconnu : rien n'est bloqué. Un n° de support inconnu du catalogue est signalé.
- Une page décochée (**Imprimer cette page**) ne se met pas non plus en file à la main.

## Voir aussi

- [Gérer la file d'une machine](rip-gerer-la-file-d-une-machine.md)
- [La sortie vers la machine a échoué](../rip-depannage/rip-sortie-vers-la-machine-echouee.md)
- [Lire le média chargé dans l'Epson](../rip-couleur/rip-lire-le-media-charge-dans-l-epson.md)
