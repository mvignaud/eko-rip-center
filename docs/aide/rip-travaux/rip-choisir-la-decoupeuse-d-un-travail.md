---
titre: Choisir la découpeuse d'un travail
description: Affectez une découpeuse à un travail et comprenez pourquoi l'impression attend tant qu'elle n'est pas utilisable.
sous_rubrique: rip-travaux
position: 62
mots_cles: découpeuse, coupe après impression, découpeuse indisponible, tracés à découper, découper seulement
---

La découpeuse choisie à l'envoi reçoit la découpe automatique de chaque page. Cet article explique son effet sur la file.

## Avant de commencer

Une découpeuse est déclarée dans le catalogue (voir [Ajouter une machine au RIP](../rip-machines/rip-ajouter-une-machine.md)).

## Étapes

1. Cliquez **Importer**.
2. Choisissez le **Mode** : **Imprimer (et découper)** ou **Découper seulement** (rien n'est imprimé).
3. Dans **Tracés à découper**, choisissez les tons de découpe reconnus ou **Tous les tracés**.
4. Dans **Découpeuse**, choisissez la machine (**— aucune —** si le travail ne se découpe pas).
5. Cliquez **Envoyer au RIP**.
6. Pour une page déjà rendue, cliquez **Découper sur…** dans l'inspecteur.

## Bon à savoir

- Tant que la découpeuse n'est pas utilisable (absente, hors ligne, mal connectée, ou une couche sans réglage sur elle), l'impression attend : rien ne part, pour ne pas gâcher d'encre ni de support (raison **Découpeuse indisponible**).
- Une découpe attend que l'impression de sa page soit finie à 100 % (raison **Impression pas finie**).
- Des travaux n'ont une planche en commun que s'ils ont la même découpeuse (« aucune » comprise).
- Avec **Dupliquer (autre jeu de repères)**, une variante de la découpeuse prend son jeu de repères : voir [Dupliquer une découpeuse pour un autre jeu de repères](../rip-machines/rip-dupliquer-une-decoupeuse-pour-un-autre-jeu-de-reperes.md).

## Voir aussi

- [Comprendre les raisons d'attente d'une sortie](rip-comprendre-les-raisons-d-attente-d-une-sortie.md)
- [Mettre les pages en file automatiquement](rip-mettre-en-file-automatiquement.md)
- [Ordonner les couches de découpe](rip-ordonner-les-couches-de-decoupe.md)
