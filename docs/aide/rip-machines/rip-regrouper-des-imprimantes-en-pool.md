---
titre: Regrouper des imprimantes en pool
description: Donnez le même nom de pool à des machines identiques pour que chaque sortie parte sur la première machine prête.
sous_rubrique: rip-machines
position: 42
mots_cles: pool, imprimantes identiques, imprimer sur pool, première machine prête, supports chargés
---

Un pool répartit les impressions entre plusieurs machines identiques. Cet article s'adresse à l'administrateur.

## Avant de commencer

Les machines ont le même modèle et la même laize, et chargent le même support.

## Étapes

1. Ouvrez **Administration…**, vue **Système**, section **Machines**.
2. Dans la fiche de chaque machine du groupe, saisissez le même nom dans **Pool**.
3. Cliquez **Enregistrer**.
4. Sur chaque page d'un travail, cliquez le bouton **Imprimer sur pool « … »**.
5. Suivez la sortie : elle se montre chez chaque membre et part sur la première machine prête, qui la prend.

## Bon à savoir

- La mise en file est refusée si les supports chargés connus des membres diffèrent : chargez le même support, ou dupliquez la machine.
- Une épreuve BAT ne se fait pas sur une machine du même pool que l'impression demandée.
- Le champ **Pool** est vide (« aucun ») par défaut.

## Voir aussi

- [Comprendre les champs d'une machine](rip-comprendre-les-champs-d-une-machine.md)
- [Gérer la file d'une machine](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
- [Valider un BAT avec le client](../rip-travaux/rip-valider-un-bat.md)
