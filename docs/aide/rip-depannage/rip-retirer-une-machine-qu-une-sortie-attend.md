---
titre: Retirer une machine qu'une sortie attend encore
description: Une machine ne peut pas être retirée du catalogue tant qu'une sortie l'attend : passez ou annulez d'abord la sortie.
sous_rubrique: rip-depannage
position: 23
mots_cles: retirer machine, supprimer machine, sortie attend, catalogue, restauration, file
---

Le message « Une sortie attend encore cette machine dans la file. » apparaît quand vous retirez une machine du catalogue alors qu'une sortie la vise toujours.

## Avant de commencer

Vous êtes administrateur. Choisissez si les sorties en attente doivent partir ou être abandonnées.

## Étapes

1. Ouvrez la vue **Production** et repérez, dans la file de la machine, les sorties encore en attente.
2. Laissez-les passer, ou annulez-les.
3. Ouvrez **Administration…**, vue **Système**, section **Machines**.
4. Retirez de nouveau la machine.

## Bon à savoir

- Le même contrôle s'applique à une restauration de sauvegarde : une machine ne peut pas disparaître du catalogue tant qu'une sortie l'attend.
- L'identifiant d'une machine nomme le dossier `sorties/<machine>/` et sert au renvoi depuis un autre RIP : ne le changez pas à la légère.

## Voir aussi

- [Ajouter une machine](../rip-machines/rip-ajouter-une-machine.md)
- [Gérer la file d'une machine](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
- [Remonter un serveur à partir d'une sauvegarde](../rip-administration/rip-remonter-un-serveur.md)
