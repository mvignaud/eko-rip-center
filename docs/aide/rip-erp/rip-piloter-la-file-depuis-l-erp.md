---
titre: Piloter la file depuis l'ERP
description: Les actions que l'ERP peut demander sur un dossier (suspendre, reprendre, annuler, priorité, échéance, opérateur, réimprimer) et leur accusé.
sous_rubrique: rip-erp
position: 31
mots_cles: actions, suspendre, reprendre, annuler, priorité, échéance, opérateur, réimprimer, travail.action, non_modifiables
---

L'ERP peut agir sur la file du RIP pour un dossier : le retenir, le passer en tête, l'annuler ou le réimprimer. Cet article décrit chaque action, sa portée et l'accusé renvoyé.

## Avant de commencer

Les actions visent les travaux nés d'un envoi de l'ERP (voir [Recevoir les travaux envoyés par l'ERP](rip-recevoir-les-travaux-envoyes-par-l-erp.md)).

## Étapes

1. Envoyez dans `actions` un élément `{ "id", "ref_erp", "action", "echeance", "operateur", "page" }`. `action` est l'une de : `suspendre`, `reprendre`, `annuler`, `priorite`, `echeance`, `operateur`, `reimprimer`.
2. Fournissez ce qui est exigé : `echeance` (secondes Unix) pour `echeance` ; `operateur` (texte de 200 caractères au plus, vide = personne) pour `operateur` ; `page` (de 1 à n) seulement pour `reimprimer`.
3. Vérifiez l'effet :
   - `suspendre` : les sorties en attente passent suspendues ; un travail du dossier qui arrive ensuite part suspendu ;
   - `reprendre` : les sorties suspendues repartent, sauf une découpe dont l'impression n'est pas terminée ; lève la retenue posée par `suspendre` ;
   - `annuler` : les sorties modifiables sont annulées, et le dossier ne part plus en file ; un travail déjà imprimé ne s'annule pas (code `deja_imprime`) ;
   - `priorite` : les sorties passent en tête de file, dans leur ordre ;
   - `echeance` : l'échéance de la fiche change, sans toucher à la file ;
   - `operateur` : l'opérateur de la fiche change, sans toucher à la file ; il apparaît dans les rapports du RIP ;
   - `reimprimer` : chaque travail rendu du dossier repart en file comme à son arrivée (même machine, découpe comprise, retenue `suspendre` respectée), sans refaire le rendu ; `page` limite à cette page.
4. Lisez l'accusé `travail.action` : `{ "action_id", "ref_erp", "action", "code", "touchees", "non_modifiables" }`.

## Bon à savoir

- **Portée** : une action vise tous les travaux nés d'un envoi de l'ERP qui portent ce `ref_erp`, et leurs sorties (impression et découpe) encore en file. Un travail créé au RIP n'est jamais touché, même s'il porte la même commande dans sa fiche.
- **Sorties modifiables** : en attente ou suspendues. Une sortie lancée, terminée, ou posée sur une planche imbriquée est laissée telle quelle et comptée dans `non_modifiables`.
- **Codes de l'accusé** : `null` si tout s'est fait ; `aucun_travail` si aucun travail de l'ERP ne porte ce `ref_erp` ; `deja_imprime` ; `action_invalide` (élément mal formé, `detail` nomme le champ) ; `rien_a_reimprimer` (aucune sortie ajoutée). `touchees` compte les sorties changées ou ajoutées, ou les travaux pour `echeance` et `operateur`.
- `reimprimer` ne double pas un travail en cours de rendu ou qui a encore des sorties en file : il est compté dans `non_modifiables`. Un dossier annulé ne repart pas : envoyez d'abord `reprendre`.
- **Doublons** : une action prise n'est jamais reprise, même reproposée ; elle est oubliée 30 jours après que l'ERP a cessé de la proposer.

## Voir aussi

- [Recevoir les travaux envoyés par l'ERP](rip-recevoir-les-travaux-envoyes-par-l-erp.md)
- [Gérer la file d'une machine](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
- [Comprendre les états des travaux et des sorties](rip-comprendre-les-etats-des-travaux-et-des-sorties.md)
