---
titre: Corriger l'estimation d'encre avec les facteurs de correction
description: Appliquez le facteur proposé par le serveur (encre réelle ÷ estimée) par machine, résolution et encre pour des estimations plus justes.
sous_rubrique: rip-couleur
position: 51
mots_cles: facteur de correction, encre estimée, encre réelle, appliquer, remettre à 1, journal
---

L'encre estimée peut s'écarter de l'encre réelle. Un facteur de correction par machine, résolution et encre rapproche les deux. Cet article s'adresse à l'administrateur.

## Avant de commencer

Il faut des impressions mesurées : par le pilote direct, avec relevé SNMP (voir [Consulter le journal des impressions](rip-consulter-le-journal-des-impressions.md)). Sans elles, l'écran affiche « Rien à proposer ».

## Étapes

1. Ouvrez **Administration…**, vue **Système**, section **Journal des impressions**, bloc **Facteurs de correction de l'encre estimée**.
2. Lisez le facteur proposé : *encre réelle ÷ encre estimée* sur les 10 dernières impressions mesurées de chaque combinaison machine, résolution et encre.
3. Cliquez **Appliquer** pour le valider.
4. Pour l'enlever, cliquez **Remettre à 1**.

## Bon à savoir

- Le facteur est de 1 par défaut.
- Il ne vaut que pour les impressions suivantes : le journal garde le facteur de chacune.
- Rétention et facteurs sont dans la sauvegarde de la configuration.

## Voir aussi

- [Consulter le journal des impressions](rip-consulter-le-journal-des-impressions.md)
- [Régler les coûts d'encre et de support](rip-regler-les-couts-d-encre-et-de-support.md)
- [Sauvegarder la configuration du serveur](../rip-administration/rip-sauvegarder-la-configuration.md)
