---
titre: Connaître les limites actuelles du RIP
description: Retrouvez ce que le RIP ne fait pas encore (blanc et argent en direct, impression Roland, cadre Kona type 3, canaux TIFF, couleurs métallisées).
sous_rubrique: rip-machines
position: 49
mots_cles: limites, pas encore, roland, blanc, argent, pilote direct, type 3, tiff, métallisé, feuille de route
---

Cet article liste ce qui n'est pas encore pris en charge, pour ne pas chercher une fonction qui n'existe pas. L'ordre de développement est public.

## Avant de commencer

Rien de particulier. La feuille de route est sur https://e-ko.rip/feuille-de-route/.

## Étapes

1. Comparez votre besoin à cette liste.

   | Ce qui n'existe pas encore | En attendant |
   |---|---|
   | Le blanc et l'argent par le pilote direct de l'Epson | Ils sortent en dossier (machine en connexion **Dossier**). |
   | L'impression par la Roland | Le RIP ne lui envoie que la découpe. |
   | Le cadre Kona type 3 (multi-segments) n'a pas encore été essayé sur la machine | Préférez le type 2. |
   | Canaux de tons directs d'un TIFF : le sens des valeurs reste à vérifier sur un vrai TIFF Photoshop | Regardez la plaque avant d'imprimer. Le canal alpha, lui, donne déjà une plaque de blanc ou de vernis. |
   | Les couleurs métallisées `MT-*` du nuancier Epson | Pas encore converties. |

2. Pour une machine absente du catalogue, voir [Connaître les machines prises en charge](rip-connaitre-les-machines-prises-en-charge.md).

## Bon à savoir

- Le blanc et l'argent par une Epson en réseau : la mise en file est refusée avec « Envoi à l'imprimante impossible » : voir [La sortie vers la machine a échoué](../rip-depannage/rip-sortie-vers-la-machine-echouee.md).
- Pour le blanc généré à partir de l'encre, voir [Poser du blanc automatiquement](../rip-couleur/rip-poser-du-blanc-automatique.md).

## Voir aussi

- [Connaître les machines prises en charge](rip-connaitre-les-machines-prises-en-charge.md)
- [Brancher la Roland VersaCAMM VP-300 / VP-540](rip-brancher-roland-versacamm.md)
- [Comprendre le pilote direct de l'Epson](rip-comprendre-le-pilote-direct-de-l-epson.md)
