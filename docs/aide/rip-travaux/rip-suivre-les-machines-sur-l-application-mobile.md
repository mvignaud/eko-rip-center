---
titre: Suivre l'état des machines sur l'application mobile
description: Lisez l'onglet Machines : état, sortie en cours, niveaux d'encre, support restant et alertes de chaque machine.
sous_rubrique: rip-travaux
position: 72
mots_cles: application mobile, machines, encre, rouleau, injoignable, alertes, snmp
---

L'onglet **Machines** montre une carte par machine, pour savoir si elle peut produire et s'il lui manque de l'encre ou du support.

## Avant de commencer

L'application est connectée au serveur.

## Étapes

1. Ouvrez l'onglet **Machines**.
2. Lisez l'état de la carte : **En impression**, **En découpe**, **En attente** ou **Injoignable**.
3. Repérez la sortie en cours.
4. Contrôlez le niveau de chaque encre : les niveaux bas sont mis en évidence.
5. Lisez la longueur de support restante sur le rouleau.
6. Lisez les alertes en cours (capot ouvert, fin de rouleau…) et l'heure du dernier relevé.

![Onglet Machines : état, encres et rouleau de chaque machine](../images/mobile-machines.png)

## Bon à savoir

- Le serveur interroge les machines Epson environ une fois par minute : les valeurs ont donc au plus quelques instants de retard.
- Une machine « Injoignable » ne répond plus au serveur : vérifiez son alimentation et son réseau sur place.

## Voir aussi

- [Suivre la file d'impression sur l'application mobile](rip-suivre-la-file-sur-l-application-mobile.md)
- [Consulter les alertes de l'application mobile](rip-consulter-les-alertes-de-l-application-mobile.md)
- [Choisir la connexion d'une machine](../rip-machines/rip-choisir-la-connexion-d-une-machine.md)
