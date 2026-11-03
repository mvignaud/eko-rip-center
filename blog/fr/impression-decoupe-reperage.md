---
titre: Impression puis découpe : repérage et contours de découpe
description: Comment faire coïncider impression et découpe : tracés CutContour, cadre et code-barres, ARMS, planches de stickers, plotters et tables. Le guide d'atelier.
date: 2026-11-03
categorie: Atelier
image: /assets/img/linux-envoi.webp
alt: Écran d'envoi d'un travail au RIP avec le réglage de découpe et la fiche du travail
---
Pour qu'une découpe tombe pile sur l'impression, le RIP doit faire trois choses : transformer les tracés de découpe du fichier en vecteurs pour la découpeuse, imprimer des repères que la machine sait lire, et n'envoyer la découpe qu'une fois la page entièrement imprimée. Dans E-KO RIP Center, ces trois étapes sont réglées au même endroit, dans le panneau d'envoi du travail.

## Qu'est-ce qu'un contour de découpe pour un RIP ?

Un contour de découpe est un tracé vectoriel peint dans un **ton direct** dont le nom signale la découpe : il n'est jamais imprimé, il est converti en vecteurs HP-GL envoyés à la découpeuse. Cette séparation est le principe du flux « impression puis découpe » : le même PDF contient l'image et le chemin de la lame.

E-KO RIP Center reconnaît un ton de découpe par le début de son nom. Les préfixes par défaut sont `CutContour`, `Thrucut`, `Kisscut`, `InsideCut`, `Perforation`, `Creasing`, `Score` et `VCut`. Les majuscules, les espaces et les tirets sont indifférents : `CutContour1` est reconnu, `1CutContour` ne l'est pas. La liste se modifie dans **Système > Découpe**.

Quelques règles à retenir côté fichier :

- le tracé de découpe est un **filet seul**, sans fond ;
- le ton est défini en **CMJN, en ton direct**, pas en RVB ;
- le filet est aligné **au centre** du tracé, pas à l'intérieur ni à l'extérieur ;
- le tracé se trouve sur le calque du dessus, pour ne pas être recouvert par l'image ;
- un ton à 0 % n'est pas découpé.

## Comment le RIP contrôle-t-il la découpe avant de couper ?

Au rendu, le RIP passe les tracés en revue et affiche des alertes sur le travail : ton défini en RVB, tracé avec un fond, tracé qui touche le bord de la page ou qui sort du plan de travail, contour vectorisé qui serait coupé deux fois, tracé recouvert par l'image, découpeuse choisie alors que le fichier ne contient aucun ton de découpe reconnu. Mieux vaut lire ces alertes avant de lancer l'impression que découvrir le problème sur le vinyle.

Le RIP rogne aussi les tracés à la page : la lame ne sort jamais du support. Le tri des tracés, actif par défaut, coupe les intérieurs d'abord.

## Comment ordonner plusieurs types de coupe ?

Le panneau **Couches de découpe** crée une couche par nom de ton. L'ordre des couches est l'ordre de coupe : le rainage et la perforation passent avant la coupe à travers, quand la matière tient encore. Chaque couche peut recevoir son propre réglage de coupe (vitesse, force, accélération). Par défaut, le réglage de la découpeuse qui porte le nom de la couche s'applique seul, sinon le premier de la machine. Une case décochée désactive la couche.

## Comment éviter le liseré blanc au bord de la découpe ?

Avec **Étendre l'image près des tracés**, l'encre qui s'arrête à moins du seuil de proximité d'un contour fermé est prolongée jusqu'au contour, puis au-delà sur une largeur réglable de 0,1 à 10 mm. La coupe ne laisse alors pas de liseré blanc. Elle ne remplace pas l'étalonnage de la découpeuse : gardez des valeurs prudentes et mesurez sur un tirage.

Pour les travaux dont le fichier n'a pas de contour, le **détourage automatique** analyse le rendu, retire le blanc autour de l'image et propose un contour `CutContour`. Un aperçu précède l'acceptation.

## Comment le repérage fait-il coïncider impression et découpe ?

Le repérage consiste à imprimer, autour de l'image, des marques que la découpeuse lit pour recaler sa lame sur l'impression. E-KO RIP Center propose pour le Mutoh Kona 1650 plusieurs formes, à choisir dans le panneau d'envoi :

| Repérage | Contenu | Usage |
|---|---|---|
| Croix P1 à P4 | Quatre croix aux angles, à 10 mm de l'image | Pointage manuel sur le Kona |
| Cadre Mutoh (type 1) | Cadre noir de 5 mm | Cadre sans code-barres, lecture non automatique |
| Cadre + code-barres (type 2) | Cadre et deux codes POSTNET avec le numéro en clair | Lecture automatique, découpe « Single Frame » |
| Cadre multi-segments (type 3) | Repères tous les 500 mm au plus | Grandes longueurs coupées par segments |

Le type 2 a été éprouvé à l'atelier : impression, lecture du cadre et des codes, puis coupe. Le type 3 suit les tableaux du guide d'application du Kona et reste à éprouver. Avec le type 2, une page de 1,5 m et plus dans le sens d'avance prend automatiquement le cadre multi-segments. Le fichier de découpe porte le nom attendu par le logiciel de la machine, du type `Nom_SF_<numéro>.plt`.

Sur un support sombre, transparent ou réfléchissant, l'option **Imprimer un fond sous les repères** pose une bande de blanc ou de CMJN sous chaque repère pour que la machine les lise.

Pour la Graphtec FC7000, le repérage ARMS se règle au panneau de la machine ; le RIP envoie la découpe en HP-GL au pas de 0,025 mm.

## Quelles machines pour quelle découpe ?

| Machine | Famille | Ce que fait le RIP |
|---|---|---|
| Mutoh Kona 1650 | Plotter de découpe | Découpe HP-GL au pas de 0,01 mm, repérage par cadre et code-barres |
| Graphtec FC7000 | Plotter de découpe | Découpe HP-GL au pas de 0,025 mm, décalage X/Y |
| Roland VersaCAMM VP-300 / VP-540 | Imprimante-découpeuse | Découpe par le réseau, en essai ; impression pas encore pilotée |

Aucune table de découpe n'est prise en charge pour l'instant. Une machine absente de la liste peut déjà passer par la connexion **Dossier** ou **IPP** ; la [liste de compatibilité](/compatibilite/) permet de demander un pilote.

## Comment préparer une planche de stickers ?

Une planche de stickers regroupe plusieurs motifs sur un même rouleau pour limiter les chutes. Dans E-KO RIP Center, on coche **Ajouter à la planche** sur chaque page avec son nombre de copies, on règle la laize et l'espacement, puis on crée la planche. Le panneau dessine l'imposition avant création, avec la longueur de rouleau et la part de surface utilisée.

Les options utiles pour les stickers :

- **Imbriquer sur la forme des contours de découpe** : chaque élément occupe la place de son contour plutôt que de son rectangle, avec des ronds en quinconce ou des triangles tête-bêche ;
- **Contour de toute la planche** (couche `NestTrimBox`) : un rectangle de découpe coupé en dernier, avec marges haute et basse pour la table de découpe ;
- **Step & repeat** : une page répétée en grille.

La planche est un travail à part entière, avec la découpe de tous les contours. L'imbrication peut aussi se faire dans la file, pour les travaux de même support, même mode et même découpeuse.

## Quelle règle d'or pour ne pas gâcher de support ?

Le RIP applique deux protections. Une découpe attend que l'impression de sa page soit finie à 100 %. Et si la découpeuse n'est pas utilisable (absente, hors ligne, mal connectée), l'impression elle-même attend : rien ne part, ni encre ni support gâchés.

Le détail de chaque réglage est dans [la documentation d'utilisation](/documentation/utilisation/), et la page [du Mutoh Kona](/documentation/machines/mutoh-kona/) dit comment le brancher.

Pour aller plus loin sur le blanc et les supports, lisez [imprimer en blanc, argent et vinyle](/blog/imprimer-blanc-argent-vinyle/).

**Essayez gratuitement** : [téléchargez E-KO RIP Center](/telecharger/) et testez votre propre flux d'impression et de découpe. Une question sur votre machine ? [Écrivez-nous](/contact/).

## Questions fréquentes

### Quel nom donner au ton de découpe dans mon PDF ?

Un nom qui commence par l'un des préfixes reconnus, par exemple `CutContour`. La liste est modifiable dans Système > Découpe, et un changement vaut pour les rendus suivants.

### Le RIP imprime-t-il les tracés de découpe ?

Non. Les tons de découpe ne partent jamais à l'imprimante : ils sont convertis en HP-GL pour la découpeuse. Le fichier .plt reste téléchargeable.

### Mon Kona doit-il lire un code-barres ?

Pour la lecture automatique, oui : prenez le cadre avec code-barres. Un cadre sans code-barres n'est pas lu seul par le Kona, qui s'arrête sur une erreur.

### Peut-on imbriquer des stickers selon leur forme ?

Oui, avec l'option d'imbrication sur la forme des contours de découpe. Un élément sans contour fermé garde son rectangle.
