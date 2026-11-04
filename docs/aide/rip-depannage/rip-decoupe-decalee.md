---
titre: Corriger une découpe décalée par rapport à l'impression
description: Mesurez l'écart sur une coupe d'essai et saisissez-le en Décalage X / Y pour recaler la découpe sur l'impression.
sous_rubrique: rip-depannage
position: 5
mots_cles: découpe décalée, décalage x y, coupe d'essai, repérage, kona, étalonnage
---

Si la découpe tombe à côté du motif imprimé, un décalage fixe de la machine suffit en général. Cet article s'adresse à l'administrateur.

## Avant de commencer

Il vous faut un compte administrateur et un support pour une coupe d'essai.

## Étapes

1. Envoyez un petit travail imprimé et découpé.
2. Mesurez l'écart entre la découpe et l'impression.
3. Dans le client, ouvrez **Administration… › Système › Machines** et la fiche de la découpeuse.
4. Saisissez l'écart en **Décalage X / Y** (de −20 à 20 mm).
5. Cliquez **Enregistrer**.
6. Refaites une coupe d'essai et ajustez si besoin.

## Bon à savoir

- Le décalage est ajouté à chaque découpe envoyée à cette machine.
- Si la coupe sort tournée de 180°, cochez **Découpe retournée (180°)**.
- Dans **Mise en coupe**, **Décaler la coupe vers l'extérieur (mm)** agrandit la pièce, mais ne remplace pas ce décalage.
- Pour le Kona, vérifiez aussi le repérage : cadre avec code-barres (type 2).
- Sur une impression qui sort courbée après les sécheurs, utilisez **Compenser le cintrage**.

## Voir aussi

- [Brancher la Mutoh Kona 1650](../rip-machines/rip-brancher-mutoh-kona-1650.md)
- [Brancher la Graphtec FC7000](../rip-machines/rip-brancher-graphtec-fc7000.md)
- [Ajouter une machine au RIP](../rip-machines/rip-ajouter-une-machine.md)
- Page détaillée : https://e-ko.rip/documentation/depannage/
