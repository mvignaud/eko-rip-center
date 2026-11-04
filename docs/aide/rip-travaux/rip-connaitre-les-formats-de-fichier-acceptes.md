---
titre: Connaître les formats de fichier acceptés
description: Vérifiez les formats, la taille maximale et le traitement des images, des canaux de tons directs et du canal alpha avant d'envoyer un fichier.
sous_rubrique: rip-travaux
position: 53
mots_cles: formats, pdf, ai, svg, tiff, png, jpeg, canal alpha, tons directs, 8 go, ppi
---

Cet article récapitule ce que le RIP accepte, ce qu'il refuse et comment il lit les images. Il s'adresse à toute personne qui prépare des fichiers.

## Avant de commencer

Rien de particulier : il suffit de connaître le format de votre fichier.

## Étapes

1. Comparez votre fichier à ce tableau.

   | Format | Particularités |
   |---|---|
   | **PDF** | Format de référence : toutes les fonctions (découpe, imbrication, filtre couleur, détourage). |
   | **AI** | À enregistrer avec « Créer un fichier compatible PDF ». |
   | **SVG** | Pour la découpe seulement. |
   | **TIFF, PNG, JPEG** | Image posée à sa taille réelle d'après la résolution enregistrée dans le fichier ; sans résolution, 72 ppi et le travail le signale. CMJN, RVB ou niveaux de gris, 8 ou 16 bits, avec son profil ICC ; la transparence ne pose pas d'encre. |

2. Vérifiez la taille : 8 Go au plus par fichier (réglage `EKO_RIP_MAX_MO` du serveur).
3. Pour un TIFF avec des canaux de tons directs, nommez-les dans Photoshop : chacun devient sa propre plaque, posée par-dessus les couleurs sans les effacer. Un canal sans nom devient `Spot1`, `Spot2`…
4. Pour obtenir de la plaque de blanc depuis un TIFF, nommez le canal `White`, `Blanc`, `WH` ou `Spot1` (majuscules, espaces et `_` ignorés).
5. Après l'envoi, regardez les plaques reprises, signalées par le travail, avant la première impression.

## Bon à savoir

- Pas encore pris en charge, refusés avec un message : TIFF à palette, YCbCr, Lab ou à plans séparés, PNG entrelacé.
- Les canaux de sélection de Photoshop (masques) ne s'impriment pas.
- Le sens des valeurs d'un canal de ton direct (canal clair = sans encre) n'a pas encore été vérifié sur un fichier de l'atelier : regardez la plaque avant d'imprimer.
- Un canal alpha (PNG ou TIFF détouré) peut donner une plaque de blanc ou de vernis : voir [Poser du blanc ou du vernis depuis un canal alpha](../rip-couleur/rip-poser-du-blanc-ou-du-vernis-depuis-un-canal-alpha.md).
- Une image sans canal alpha (JPEG) n'a pas cette plaque.
- Si le fichier dépasse la limite, le message est « Fichier trop volumineux pour le serveur. » : voir [Résoudre un rendu en échec ou un fichier refusé](../rip-depannage/rip-un-rendu-echoue.md).

## Voir aussi

- [Envoyer un travail au RIP](rip-envoyer-un-travail.md)
- [Comprendre les plaques d'encre et les noms de tons directs](../rip-couleur/rip-comprendre-les-plaques-et-les-tons-directs.md)
- [Lire les alertes d'un travail](rip-lire-les-alertes-d-un-travail.md)
