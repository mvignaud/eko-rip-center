---
titre: Le Kona s'arrête sur « EPOS 6213 »
description: Comprenez pourquoi le Kona refuse de lire un cadre sans code-barres et passez au cadre de type 2.
sous_rubrique: rip-depannage
position: 40
mots_cles: epos 6213, kona, cadre, code-barres, type 1, repérage, mutoh
---

Le Kona s'arrête et affiche « EPOS 6213 » au lieu de couper. Cet article donne la cause et la correction, pour les opérateurs.

## Avant de commencer

Le travail a été imprimé avec un **Repérage Kona** et envoyé directement au Kona.

## Étapes

1. Vérifiez le repérage du travail : cliquez le travail, puis **Réglages**, et lisez **Repérage Kona**.
2. Si c'est **Cadre Mutoh (type 1)**, c'est la cause : le Kona ne lit pas seul un cadre sans code-barres (manuel, § 6.2).
3. Refaites le rendu avec **Cadre Mutoh + code-barres (type 2, CutServer SF)** : cliquez **Appliquer et refaire le rendu**.
4. Réimprimez, puis déposez le fichier `Nom_SF_<numéro>.plt` dans le dossier de CutServer : le Kona lit le code et CutServer envoie le fichier du même numéro.

## Bon à savoir

- Le type 2 est celui éprouvé à l'atelier.
- Pour une page de 1,5 m et plus dans le sens d'avance, le type 2 prend seul le cadre multi-segments (type 3).
- Si la découpe sort décalée plutôt que de s'arrêter, voir [Corriger une découpe décalée par rapport à l'impression](rip-decoupe-decalee.md).

## Voir aussi

- [Choisir le repérage Kona](../rip-machines/rip-choisir-le-reperage-kona.md)
- [Brancher la Mutoh Kona 1650](../rip-machines/rip-brancher-mutoh-kona-1650.md)
- [Corriger une découpe décalée par rapport à l'impression](rip-decoupe-decalee.md)
