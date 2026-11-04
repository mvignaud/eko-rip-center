---
titre: Régler la coupe du Kona
description: Déclarez les réglages de coupe (vitesse, force, accélération, outil, motif), récupérez le fichier .plt tel que le Kona le reçoit et retournez la coupe.
sous_rubrique: rip-machines
position: 47
mots_cles: réglages de coupe, vitesse, force, accélération, outil, motif, diffusion, plt, découpe retournée, massicot
---

Comme dans Mutoh CutServer, chaque couche de découpe part avec son réglage de coupe. Cet article s'adresse à l'administrateur.

## Avant de commencer

Le Kona est branché (voir [Brancher la Mutoh Kona 1650](rip-brancher-mutoh-kona-1650.md)). Il vous faut un compte administrateur.

## Étapes

1. Ouvrez **Administration…**, vue **Système**, section **Machines**, fiche du Kona.
2. Cliquez **Ajouter un réglage** (sous **Réglages de coupe**) et donnez son nom (le nom d'une couche de découpe, pour l'appliquer seul).
3. Saisissez la **Vitesse (cm/s)**, la **Force (g)** et l'**Accélération (G)**.
4. Au besoin, cochez **Valeurs du massicot** (rien n'est envoyé : le Kona garde ses valeurs), imposez un **Outil** (1 à 4) ou laissez **par défaut**.
5. Pour un motif de découpe, saisissez le **Motif (bas/haut, mm)** : jusqu'à 10 paires « bas/haut » en mm (`8/2 8/2` = couper 8 mm, lever 2 mm), et cochez **Diffusion du motif** pour un nombre entier de motifs par ligne droite (la coupe commence et finit aux angles vifs).
6. Cliquez **Enregistrer**. Le premier réglage sert par défaut.

## Bon à savoir

- Pour récupérer le fichier tel que le Kona le reçoit, à envoyer à la main : **Fichier .plt** avec `?machine=kona` (port série 9600, 8N1, RTS/CTS).
- Si la coupe sort tournée de 180°, cochez **Découpe retournée (180°)** sur la machine.
- Avec un cadre, le RIP déclare les deux coins opposés du cadre (milieux des traits), le Kona le balaie, lit le code, puis coupe.
- Chaque couche de découpe part avec son propre réglage (voir [Ordonner les couches de découpe](../rip-travaux/rip-ordonner-les-couches-de-decoupe.md)).
- Le Kona doit être réglé sur une résolution de **0,010 mm** ; le pas de découpe de la machine est 0,01 mm.

## Voir aussi

- [Choisir le repérage Kona](rip-choisir-le-reperage-kona.md)
- [Ordonner les couches de découpe](../rip-travaux/rip-ordonner-les-couches-de-decoupe.md)
- [Corriger une découpe décalée par rapport à l'impression](../rip-depannage/rip-decoupe-decalee.md)
