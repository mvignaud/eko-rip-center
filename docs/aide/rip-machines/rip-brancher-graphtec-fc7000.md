---
titre: Brancher la Graphtec FC7000
description: Branchez la Graphtec FC7000 sur le réseau et passez-la en HP-GL pour que le RIP lui envoie la découpe.
sous_rubrique: rip-machines
position: 6
mots_cles: graphtec, fc7000, plotter, hp-gl, arms, découpe, réseau
---

Le RIP envoie la découpe à la Graphtec FC7000 en HP-GL, au pas de 0,025 mm.

## Avant de commencer

Il vous faut un compte administrateur. La machine est en réseau avec une adresse fixe, ou branchée en série ou USB-série.

## Étapes

1. Sur la Graphtec, réglez les commandes en **HP-GL** : Menu > Interface > Command.
2. Réglez le repérage **ARMS** au panneau de la machine.
3. Réservez une adresse fixe pour la machine.
4. Dans le client, ouvrez **Administration… › Système › Machines** et ajoutez la Graphtec FC7000.
5. Choisissez la connexion **Réseau (TCP)** (ou série / USB-série) et saisissez l'adresse et le port.
6. Vérifiez le pas de découpe (0,025 mm), les réglages de coupe et le **Décalage X / Y**, puis enregistrez.
7. Vérifiez depuis le serveur : `nc -vz <adresse-de-la-graphtec> <port>`.

## Bon à savoir

- Rien ne coupe : l'interface de la machine n'est pas en HP-GL.
- Sortie en échec, machine injoignable : vérifiez l'adresse, le port et le câble.
- Si la découpe est décalée par rapport à l'impression, mesurez l'écart sur une coupe d'essai et saisissez le **Décalage X / Y**.

## Voir aussi

- [Ajouter une machine au RIP](rip-ajouter-une-machine.md)
- [La découpe est décalée par rapport à l'impression](../rip-depannage/rip-decoupe-decalee.md)
- [Choisir la connexion d'une machine](rip-choisir-la-connexion-d-une-machine.md)
- Page détaillée : https://e-ko.rip/documentation/machines/graphtec-fc7000/
