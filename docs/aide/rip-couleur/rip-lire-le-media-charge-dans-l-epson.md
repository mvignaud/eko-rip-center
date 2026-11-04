---
titre: Lire le média chargé et la consommation réelle de l'Epson
description: Lisez la largeur du rouleau, le support, les encres et les alertes de l'Epson par le réseau, et suivez la consommation sur sept jours.
sous_rubrique: rip-couleur
position: 47
mots_cles: média chargé, snmp, laize, rouleau, encres, consommation réelle, utiliser comme laize, relire
---

Le RIP lit l'Epson par le réseau pour connaître le rouleau chargé et relever la consommation. Cet article s'adresse à l'administrateur.

## Avant de commencer

L'Epson est branchée au réseau (voir [Brancher l'Epson SureColor SC-S80600](../rip-machines/rip-brancher-epson-sc-s80600.md)). Renseignez l'adresse de l'Epson dans `EKO_RIP_EPSON_SNMP` (fichier `/etc/default/eko-rip-serveur`) pour cet écran, puis redémarrez le service.

## Étapes

1. Ouvrez **Administration…**, vue **Système**, section **Média chargé dans l'Epson**.
2. Cliquez **Relire** pour relever l'imprimante : l'écran montre la largeur du rouleau, la longueur restante, le n° de support, les chauffages, le niveau de chaque encre, les alertes et le support chargé.
3. Cliquez **Utiliser comme laize** pour reprendre la largeur mesurée comme laize des panneaux Planche et Mosaïque.
4. Pour changer de support à la main, choisissez-le dans la liste du support chargé ; revenez à **— lu sur l'imprimante —** pour reprendre la lecture.
5. Lisez la **consommation réelle** sur 7 jours affichée à l'écran.

## Bon à savoir

- La largeur du rouleau est lue par le réseau (SNMP, Printer-MIB normalisée) ; validé sur la SC-S80600.
- Le serveur relève l'Epson chaque minute (encres et rouleau) et garde chaque changement dans `releves/epson.jsonl` (dossier des données).
- L'API `GET /api/machines/epson/consommation?depuis=&jusqua=` (secondes Unix) donne la consommation pour toute période.
- Seules les baisses comptent : un changement de poche ou de rouleau n'est pas compté comme une consommation.
- L'imprimante ne donne l'encre qu'au % près : le chiffre est fiable sur la durée, pas pour un petit travail.
- Pas de largeur de média : vérifiez l'adresse SNMP, que l'Epson est allumée et sur le même réseau, et la communauté (`EKO_RIP_EPSON_COMMUNAUTE`, `public` par défaut).

## Voir aussi

- [Régler un support et ses modes d'impression](rip-regler-un-support-et-ses-modes.md)
- [Consulter le journal des impressions](rip-consulter-le-journal-des-impressions.md)
- [Brancher l'Epson SureColor SC-S80600](../rip-machines/rip-brancher-epson-sc-s80600.md)
