---
titre: Poser du blanc ou du vernis depuis un canal alpha
description: Faites suivre exactement le détourage d'un PNG ou d'un TIFF par une plaque White ou Varnish tirée de son canal alpha.
sous_rubrique: rip-couleur
position: 41
mots_cles: canal alpha, white, varnish, vernis sélectif, png, tiff, détourage, plaque
---

Un PNG ou un TIFF détouré porte un canal alpha qui dit où l'image est opaque. Le RIP peut en tirer une plaque de blanc ou de vernis sélectif. Cet article s'adresse aux opérateurs.

## Avant de commencer

Votre fichier est un PNG ou un TIFF avec un canal alpha (un JPEG n'en a pas).

## Étapes

1. Cliquez **Importer** et choisissez le fichier.
2. Ouvrez le bloc **Blanc**.
3. Dans le réglage **Canal alpha**, choisissez une plaque `White` (blanc) ou `Varnish` (vernis sélectif).
4. Si besoin, réglez aussi le blanc généré (sous-couche, aplat) : il s'ajoute à la plaque du canal alpha.
5. Cliquez **Envoyer au RIP**, puis ouvrez **Couches** pour contrôler la plaque.

## Bon à savoir

- La plaque suit exactement le détourage : opaque = pleine encre, bord adouci compris, sans rétraction.
- Un alpha partout opaque donne une plaque sur toute l'image.
- Si le TIFF a déjà un canal de ton direct du même nom, c'est ce canal qui compte.
- Une image sans canal alpha (JPEG) n'a pas cette plaque.
- Pour le blanc généré à partir de l'encre, voir [Poser du blanc automatiquement](rip-poser-du-blanc-automatique.md).

## Voir aussi

- [Poser du blanc automatiquement](rip-poser-du-blanc-automatique.md)
- [Connaître les formats de fichier acceptés](../rip-travaux/rip-connaitre-les-formats-de-fichier-acceptes.md)
- [Comprendre les plaques d'encre et les noms de tons directs](rip-comprendre-les-plaques-et-les-tons-directs.md)
