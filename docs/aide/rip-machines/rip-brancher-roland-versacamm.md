---
titre: Brancher la Roland VersaCAMM VP-300 / VP-540
description: Branchez la Roland VersaCAMM sur le réseau pour que le RIP lui envoie la découpe.
sous_rubrique: rip-machines
position: 4
mots_cles: roland, versacamm, vp-300, vp-540, découpe, imprimante-découpeuse, réseau
---

La Roland VersaCAMM est une imprimante-découpeuse. Le RIP lui envoie la découpe par le réseau (laize 762 ou 1371 mm). L'impression par la Roland n'est pas encore pilotée par le RIP.

## Avant de commencer

Il vous faut un compte administrateur et la Roland branchée au réseau du serveur avec une adresse fixe.

## Étapes

1. Réservez une adresse fixe pour la Roland sur votre routeur.
2. Dans le client, ouvrez **Administration… › Système › Machines** et cliquez **Ajouter**.
3. Choisissez le modèle Roland VersaCAMM VP et la laize (762 ou 1371 mm).
4. Choisissez la connexion **Réseau (TCP)** et saisissez l'adresse et le port de la machine.
5. Enregistrez.
6. Vérifiez depuis le serveur : `nc -vz <adresse-de-la-roland> <port>`.

## Bon à savoir

- La prise en charge de la Roland est récente : signalez tout écart avec **Signaler un problème…** dans le client.
- Si la découpe est décalée par rapport à l'impression, réglez le **Décalage X / Y** de la machine.
- Sortie en échec « machine injoignable » : vérifiez l'adresse, le port et le câble.

## Voir aussi

- [Ajouter une machine au RIP](rip-ajouter-une-machine.md)
- [La découpe est décalée par rapport à l'impression](../rip-depannage/rip-decoupe-decalee.md)
- [Choisir la connexion d'une machine](rip-choisir-la-connexion-d-une-machine.md)
- Page détaillée : https://e-ko.rip/documentation/machines/roland-versacamm/
