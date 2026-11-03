---
titre: Étalonnage G7 et linéarisation : guide d'atelier
description: Étalonnage du gris type G7, linéarisation par encre, profils ICC de support et courbes : la méthode pas à pas pour une couleur stable en grand format.
date: 2026-11-03
categorie: Couleur
image: /assets/img/reglage-epreuve.webp
alt: Réglages d'épreuve et de couleur dans le panneau d'envoi d'E-KO RIP Center
---
Pour une couleur stable en grand format, on étalonne dans cet ordre : régler la limite d'encre, linéariser chaque encre, étalonner le gris, puis seulement profiler le support avec un profil ICC. Chaque étape corrige un défaut différent, et sauter l'une d'elles rend les suivantes instables.

## Pourquoi étalonner avant de profiler ?

Une imprimante dérive : tête, température, lot d'encre, support. Un profil ICC décrit l'état de la machine **à un instant donné**. S'il est fait sur une machine qui dérive, il se périme vite. L'étalonnage ramène la machine à un comportement connu et répétable ; le profil sert ensuite à convertir les couleurs depuis cet état stable.

L'ordre est donc : limite d'encre, linéarisation, étalonnage du gris, profil ICC, courbes de retouche au besoin.

> [!NOTE]
> Dans E-KO RIP Center, l'étalonnage du gris de type G7 et la linéarisation se font aujourd'hui par l'API du serveur, réservée à l'administrateur. La méthode et les seuils sont les nôtres : ce n'est pas une certification G7. Les courbes calculées s'appliqueront au rendu avec la linéarisation par mode, prévue ensuite.

## Qu'est-ce que la linéarisation ?

La **linéarisation** mesure des plages d'encre pour régler la réponse de chaque encre sur un support. Sans elle, 50 % demandé au RIP ne donne pas 50 % de densité perçue : la réponse est courbe, avec des hautes lumières qui s'engraissent trop vite ou des ombres bouchées.

Le principe : imprimer une mire d'une plage par encre et par palier, mesurer chaque plage au spectrophotomètre, puis corriger pour que la progression soit régulière. E-KO RIP Center fournit une mire de linéarisation au format CGATS, resserrée dans les hautes lumières (8 plages sous 20 % sur 21, contre 4 pour une rampe régulière), car l'œil y est le plus sensible.

## Comment fixer la limite d'encre ?

Avant la linéarisation, il faut savoir quelle quantité d'encre le support accepte. Le RIP propose une aide :

1. imprimer des aplats d'encre totale croissante, par exemple de 100 à 400 % par pas de 20 % ;
2. mesurer leur luminosité L* ;
3. envoyer la série au serveur, qui propose la limite : le dernier palier qui fonce encore d'au moins 1 L* par 10 % d'encre en plus, arrondi aux 5 % inférieurs ;
4. relire les gains renvoyés, vérifier l'aplat au séchage, puis saisir la limite dans le mode d'impression.

Un palier qui s'éclaircit (bronzage, bavure) arrête la recherche. Rien n'est enregistré automatiquement : la décision reste à l'opérateur. La limite se règle par support et par mode, dans **Système > Supports de l'Epson**.

## Qu'est-ce que l'étalonnage du gris de type G7 ?

La méthode G7 vise un **équilibre des gris** : une rampe de gris composée de cyan, magenta et jaune à parts égales doit paraître neutre, avec la même courbe de tonalité qu'une rampe noire. Si le gris CMJ vire au rouge ou au vert, aucune retouche couleur par couleur ne tiendra durablement.

Le déroulé dans E-KO RIP Center :

1. récupérer la mire, une rampe CMJ à parts égales et une rampe noire, de 0 à 100 % ;
2. l'imprimer sur le support et le mode concernés ;
3. mesurer en L*a*b* chaque plage ;
4. envoyer les mesures au serveur (chaque rampe demande son papier à 0 %, son aplat à 100 % et au moins trois plages entre les deux) ;
5. lire la réponse : écarts à la cible (ΔL* moyen et écart de gris moyen), courbes de correction C, M, J et N de 5 en 5 %, historique des itérations et numéro de la meilleure.

Le résultat est `conforme` sous 1,5 pour chacun des deux écarts, des seuils indicatifs. La cible garde la teinte du papier, qui s'efface à mesure que le gris fonce.

## Pourquoi plusieurs itérations ?

On imprime la mire suivante **avec les courbes de la meilleure itération**, on mesure, puis on envoie les nouvelles mesures en indiquant quelle itération a servi. La nouvelle correction se compose avec les précédentes. Une itération moins bonne n'efface pas la meilleure, et les 50 dernières sont gardées. En général, deux ou trois passes suffisent pour se rapprocher de la cible, mais cela dépend de la machine et du support.

## Où entrent les profils ICC de support ?

Une fois la machine stable, le **profil ICC de support** décrit le gamut et le comportement de ce support avec cette encre. Dans le bloc **Couleur** du panneau d'envoi :

- **Ajouter un profil ICC…** dépose un profil `.icc` ou `.icm` sur le serveur ;
- **Entrée CMJN / Entrée RVB** s'applique aux données sans profil (ou à toutes, avec **Ignorer les profils du PDF**) ;
- **Support (sortie)** est le profil du support imprimé ; l'aperçu simule son rendu ;
- **Intention** : perceptuelle, colorimétrique relative, saturation ou absolue.

Pour les teintes de marque, la **table des tons directs** convertit un ton nommé d'après sa valeur Lab, via le profil de sortie et l'intention du travail. Par défaut, l'intention des tons directs est la colorimétrique relative, qui vise la teinte. Les tons `White` et `Silver` ne sont jamais convertis. Un profil se fait avec une mire et un spectrophotomètre par un outil de profilage externe ; le RIP l'applique.

## Comment retoucher une couleur avec les courbes ?

Une fois étalonné et profilé, il reste parfois un travail précis à corriger. La visionneuse offre un **filtre couleur**, propre au travail :

- **courbes** C, M, J, N par points « entrée:sortie » en pourcentage (par exemple `0:0 50:60 100:100`) ;
- **luminosité** et **saturation** de −100 à 100 % ;
- **remplacement de couleur** d'une teinte CMJN vers une autre, avec une plage ;
- **avant/après** en aperçu scindé.

Le filtre agit après le profil de sortie et avant le blanc automatique ; le blanc, l'argent et la découpe ne sont pas touchés. Un bouton **Exporter vers le réglage** le range dans un réglage enregistré.

## Quelles bonnes pratiques d'atelier ?

- Étalonnez **par support et par mode d'impression**, jamais globalement.
- Refaites une mesure rapide régulièrement, et après tout changement de lot d'encre, de tête ou de support.
- Attendez le séchage complet avant de mesurer.
- Gardez le même instrument et les mêmes conditions de mesure.
- Le RIP permet d'exporter un **paquet support** (support, modes et réglages) pour le reprendre sur un autre serveur, sans les profils ICC.
- La sauvegarde de configuration du serveur contient les profils ICC déposés.

Pour la logique générale d'un RIP, relisez [ce qu'est un RIP](/blog/qu-est-ce-qu-un-rip/) ; pour les encres spéciales, le blanc et l'argent. Les réglages de supports sont décrits dans la [documentation d'utilisation](/documentation/utilisation/).

## À vous de mesurer

[Installez E-KO RIP Center gratuitement](/telecharger/) et testez la mire sur votre support. Une question de méthode ou de machine ? [Écrivez-nous](/contact/).

## Questions fréquentes

### Quelle différence entre linéarisation et profil ICC ?

La linéarisation règle la réponse de chaque encre pour que la machine soit régulière. Le profil ICC décrit les couleurs que le support peut reproduire pour les convertir. On linéarise d'abord, on profile ensuite.

### L'étalonnage d'E-KO RIP Center est-il une certification G7 ?

Non. La méthode et les seuils sont ceux d'E-KO RIP Center, inspirés de l'approche G7 par équilibre des gris. Ce n'est pas une certification.

### À quelle fréquence faut-il étalonner ?

Cela dépend de la machine et du volume. Refaites au minimum une vérification après un changement de lot d'encre, de tête ou de support, puis à intervalle régulier selon vos tolérances.

### Faut-il un spectrophotomètre ?

Oui : les mesures L*a*b* des plages sont indispensables pour la limite d'encre, la linéarisation et l'étalonnage du gris.
