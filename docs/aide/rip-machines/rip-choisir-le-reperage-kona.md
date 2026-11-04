---
titre: Choisir le repérage Kona
description: Choisissez entre croix P1-P4, cadre Mutoh simple, cadre avec code-barres ou cadre multi-segments pour que le Kona lise le travail imprimé et coupe.
sous_rubrique: rip-machines
position: 46
mots_cles: repérage kona, cadre mutoh, code-barres, cutserver, croix p1 p4, multi-segments, epos 6213, flèche
---

Le repérage ajoute autour de l'image les repères que le Kona lit pour couper au bon endroit. Cet article aide à choisir le bon type. Il s'adresse aux opérateurs.

## Avant de commencer

Le travail est en mode **Imprimer** et la découpeuse est le Kona. Le Kona est réglé sur une résolution de **0,010 mm** (voir [Brancher la Mutoh Kona 1650](rip-brancher-mutoh-kona-1650.md)).

## Étapes

1. Cliquez **Importer** et choisissez le **Repérage Kona** :
   - **Croix P1–P4** : quatre croix aux coins, à 10 mm de l'image. Sur le Kona : Actions > Contour Cut > Manual, pointez P1, P2, P3, P4.
   - **Cadre Mutoh (type 1)** : cadre noir de 5 mm, à 10 mm de l'image, intérieur d'au moins 250 × 250 mm, blancs de 25 mm sur les côtés, 30 mm à l'avant, 80 mm à l'arrière (cotes minimales du manuel Kona respectées).
   - **Cadre Mutoh + code-barres (type 2, CutServer SF)** : le même cadre et deux codes POSTNET de 250 × 20 mm (bas à gauche, haut à droite, à 50 mm du côté et 9 mm du cadre) avec le numéro en clair. Intérieur d'au moins 310 mm de large. C'est le type éprouvé à l'atelier (impression, lecture du cadre et des codes, coupe).
   - **Cadre multi-segments + code-barres (type 3, CutServer MF)** : angles de 125 mm, un repère de 30 mm tous les 500 mm au plus sur chaque côté, mêmes codes, pour les grandes longueurs découpées par segments.
2. Pour le type 2 ou 3, téléchargez le fichier de découpe : il porte le nom attendu par Mutoh CutServer, `Nom_SF_<numéro>.plt` (type 2) ou `Nom_MF_<numéro>.plt` (type 3). Le numéro (11 chiffres) vient du travail et de la page.
3. Déposez le fichier dans le dossier de CutServer : le Kona lit le code et CutServer envoie le fichier du même numéro.
4. Engagez dans le Kona le bord que montre la flèche ▲ du cadre (coin intérieur bas-droit) : le bord qui sort le premier de l'Epson, engagé par l'arrière.

## Bon à savoir

- ⚠️ Le Kona ne lit pas seul un cadre sans code-barres (manuel, § 6.2) : un type 1 envoyé directement s'arrête sur « EPOS 6213 ». Pour la lecture automatique, prenez le type 2.
- **Multi-cadre automatique** : avec le type 2, une page de **1,5 m et plus** dans le sens d'avance prend d'elle-même le cadre multi-segments (type 3), et son fichier le suffixe `MF`. Le manuel Kona (§ 6.3) déconseille le cadre simple au-delà de 2 m et le multi-cadre sous 1,5 m.
- Les types 2 et 3 suivent les tableaux du guide d'application Kona ; la place du code du haut et l'écart des repères du type 3 sont lus sur ses schémas, sans cote. Le type 3 reste à éprouver sur la machine.
- **Alertes de repérage** : une image de moins de 25 cm de côté est signalée (taille conseillée pour le repérage Kona).
- Avec un repérage Kona, les traits de coupe et les croix ne sont pas imprimés (voir [Poser des œillets et des marques imprimées](../rip-travaux/rip-poser-des-oeillets-et-des-marques-imprimees.md)).
- La page imprimée est agrandie d'autant, et le fichier de découpe suit le même décalage.
- Sur un support sombre, transparent ou réfléchissant, voir [Imprimer un fond sous les repères](../rip-couleur/rip-imprimer-un-fond-sous-les-reperes.md).

## Voir aussi

- [Régler la coupe du Kona](rip-regler-la-coupe-du-kona.md)
- [Brancher la Mutoh Kona 1650](rip-brancher-mutoh-kona-1650.md)
- [Imprimer un fond sous les repères](../rip-couleur/rip-imprimer-un-fond-sous-les-reperes.md)
