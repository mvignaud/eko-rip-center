---
titre: Comprendre les états des travaux et des sorties
description: Les états d'un travail (rendu) et d'une sortie (impression ou découpe) publiés pour l'ERP, ce qu'ils veulent dire et ce qui se passe après une coupure.
sous_rubrique: rip-erp
position: 27
mots_cles: états, travail, sortie, en_attente, en_cours, termine, echec, suspendue, annulee, imbriquee, interrompu
---

Le RIP publie pour l'ERP l'état de chaque travail et de chaque sortie. Cet article décrit ces états et leur correspondance conseillée côté ERP. Ils sont aussi lisibles par programme dans `GET /api/erp/contrat` (`etats`).

## Avant de commencer

Un **travail** est le rendu d'un fichier ; une **sortie** est l'envoi d'une page à une machine (impression ou découpe). Voir le [glossaire](../rip-premiers-pas/rip-consulter-le-glossaire.md).

## Étapes

1. Lisez les états d'un **travail** :

   | État | Sens |
   |---|---|
   | `en_attente` | reçu, en attente du rendu |
   | `en_cours` | rendu en cours |
   | `termine` | rendu terminé : prêt à sortir |
   | `echec` | rendu impossible (code d'erreur stable) |
2. Lisez les états d'une **sortie** :

   | État | Sens |
   |---|---|
   | `en_attente` | en file vers une machine |
   | `suspendue` | retenue par un opérateur |
   | `en_cours` | envoi à la machine en cours |
   | `terminee` | envoyée à la machine (impression ou découpe) |
   | `echec` | envoi impossible (code d'erreur stable) |
   | `annulee` | retirée avant envoi |
3. Pour une sortie posée sur une planche d'imbrication, retenez l'état `imbriquee` : la planche, un travail à part, l'imprime, et la sortie passe `terminee` quand la planche est imprimée. La consommation est alors portée par la planche.
4. Appliquez la correspondance suggérée côté ERP : travail `termine` : « prêt à imprimer » ; sortie `terminee` d'une imprimante : « en impression » ; sortie `terminee` d'une découpeuse : « découpé » ; `echec` : « bloqué », avec le code.

## Bon à savoir

- Après une coupure, un travail `en_cours` passe en `echec` avec le code `interrompu` : il est à relancer, il ne repart pas seul. Une sortie `en_cours` passe aussi en `echec` avec `interrompu` : une découpe ne se reprend pas au milieu.
- Pour l'instant, `terminee` signifie « confiée à la machine », pas « sortie de la machine ».
- Un état nouveau documenté est un ajout compatible : l'ERP doit ignorer ceux qu'il ne connaît pas.
- Les changements d'état sont publiés par les événements décrits dans [Consulter les événements envoyés à l'ERP](rip-consulter-les-evenements-envoyes-a-l-erp.md).

## Voir aussi

- [Consulter les événements envoyés à l'ERP](rip-consulter-les-evenements-envoyes-a-l-erp.md)
- [Reprendre après une coupure de courant](../rip-administration/rip-reprendre-apres-une-coupure-de-courant.md)
- [Gérer la file d'une machine](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
