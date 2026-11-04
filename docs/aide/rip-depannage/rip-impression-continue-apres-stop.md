---
titre: L'impression continue après Stop dans l'application mobile
description: Pourquoi Stop n'arrête pas une impression déjà partie vers la machine, et comment l'interrompre.
sous_rubrique: rip-depannage
position: 73
mots_cles: stop, pause, application mobile, impression en cours, interrompre
---

Vous avez touché **Stop** dans l'application, mais la machine continue d'imprimer.

## Avant de commencer

Repérez la sortie dans l'onglet **File** et son état.

## Étapes

1. Vérifiez l'état de la sortie : **Stop** annule une sortie **En attente** ou **Suspendue**.
2. Si la sortie est **En cours**, elle est déjà partie vers la machine : interrompez-la sur la machine elle-même.
3. Une fois la machine arrêtée, relancez la sortie si besoin avec **Relancer**.

## Bon à savoir

- **Pause** et **Stop** n'agissent que sur une sortie qui n'est pas encore partie vers la machine : le serveur ne sait pas encore interrompre une impression en cours.
- Une sortie imbriquée dans une planche n'a aucune action dans l'application.

## Voir aussi

- [Agir sur une sortie depuis l'application mobile](../rip-travaux/rip-agir-sur-une-sortie-depuis-l-application-mobile.md)
- [Gérer la file d'une machine](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
- [La sortie vers la machine a échoué](rip-sortie-vers-la-machine-echouee.md)
