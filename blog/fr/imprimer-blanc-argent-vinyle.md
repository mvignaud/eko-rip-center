---
titre: Imprimer en blanc et argent : préparer ses fichiers
description: Sous-couche, sur-couche, tons directs White et Silver, rétraction : comment préparer vos fichiers et régler le RIP pour imprimer du blanc et de l'argent.
date: 2026-11-03
categorie: Couleur
image: /assets/img/reglage-blanc.webp
alt: Réglage du blanc automatique dans le panneau d'envoi d'E-KO RIP Center
---
Pour imprimer du blanc ou de l'argent, nommez un ton direct `White` ou `Silver` dans votre fichier, ou laissez le RIP générer le blanc en sous-couche ou en aplat. Le RIP en fait une plaque séparée qui part avec les autres encres. Tout se joue donc à deux endroits : le nom du ton dans le fichier et le réglage Blanc à l'envoi.

## Qu'est-ce qu'une plaque de blanc ou d'argent ?

Une imprimante qui possède des encres blanche ou métallisée les traite comme des canaux supplémentaires. Dans le fichier, on les décrit avec un **ton direct** (spot colour) : une couleur nommée qui n'est pas mélangée au CMJN et qui devient sa propre **plaque**.

Dans E-KO RIP Center, une plaque est créée par encre (Cyan, Magenta, Jaune, Noir) plus une par ton direct du PDF. Sur l'aperçu d'une plaque, le noir représente l'encre et le blanc l'absence d'encre.

> [!NOTE]
> Le blanc et l'argent ne passent pas encore par le pilote direct de l'Epson SC-S80600 : la mise en file vers une Epson en connexion Réseau est refusée. Ils sortent aujourd'hui par une machine en connexion Dossier, qui dépose un fichier par canal pour un logiciel tiers. Voir la [fiche de l'Epson](/documentation/machines/epson-sc-s80600/).

## Comment nommer les tons directs dans le fichier ?

Le nom fait tout. Pour l'Epson SC-S80600, la documentation indique :

- **Blanc** : nommer le ton direct `White` dans Illustrator ;
- **Argent métallisé** : nommer le ton direct `Silver`.

Le RIP reconnaît aussi d'autres alias : le blanc sous `White`, `Blanc` ou `WH`, l'argent sous `Metallic`, `Silver`, `Argent` ou `MS`. Epson conseille `White` et `Metallic`. Les majuscules, les espaces et les `_` sont ignorés pour les canaux d'un TIFF.

Quelques règles pratiques :

1. Créez le ton direct avec le bon nom **avant** de dessiner les objets qui l'utilisent.
2. Appliquez-le en aplat (100 %) sur les zones à imprimer. Une teinte à 50 % pose la moitié de la recette.
3. Ne mélangez pas blanc et argent sur une même page : la mise en file est refusée avec la raison.
4. Évitez les tons directs inconnus : un ton qui n'est ni une encre, ni un ton de découpe, ni dans la table est signalé, et la mise en file échoue s'il n'a pas d'encre sur la machine.

Les tons de découpe (`CutContour` et équivalents) ne sont jamais imprimés et n'effacent pas l'encre sous eux : pas de liseré blanc le long de la découpe.

## Sous-couche ou sur-couche : que choisir ?

Le blanc se place dans un ordre qui dépend du support.

| Cas | Position du blanc | Exemple |
|---|---|---|
| Support transparent ou coloré, image vue côté imprimé | Sous la couleur (sous-couche) | Vinyle transparent, support teinté |
| Image vue à travers le support | Sur la couleur (sur-couche) | Vitrophanie, impression en contre-collé |
| Fond uni derrière l'impression | Aplat sur toute la page | Fond blanc pleine page |

Dans la section Blanc du panneau d'envoi, **Blanc ou argent** pose le ton sous la couleur ou dessus (vitrophanie). Choisissez la position à l'envoi, puis contrôlez-la sur l'aperçu et sur une chute.

## Comment régler le blanc automatique ?

Plutôt que de dessiner le blanc dans le fichier, vous pouvez le faire générer. Le bloc **Blanc** (mode Imprimer) propose :

- **Sous-couche sous l'encre** : du blanc partout où une encre dépasse l'**encre minimale** (5 % par défaut) ;
- **Aplat sur toute la page** : du blanc sur la page entière ;
- **Rétraction** : le blanc recule depuis le bord de l'encre et de la page (0,2 mm par défaut, jusqu'à 5 mm).

La rétraction évite le liseré blanc qui dépasse de la couleur à cause du moindre décalage. Le blanc déjà dessiné dans le fichier est conservé : le blanc final réunit les deux. La plaque générée apparaît dans les aperçus, compte dans le coût et part avec les autres plaques. Un réglage enregistré garde ce choix, ce qui évite de le refaire à chaque travail.

## Peut-on tirer le blanc du canal alpha d'une image ?

Oui, pour un PNG ou un TIFF détouré. Le réglage « Canal alpha » de la section Blanc donne une plaque `White` (blanc) ou `Varnish` (vernis sélectif) qui suit exactement le détourage : opaque égale pleine encre, bord adouci compris, sans rétraction. Une image sans canal alpha, comme un JPEG, n'a pas cette plaque. Le blanc généré s'y ajoute s'il est choisi aussi.

Pour un TIFF multicanal, chaque canal de ton direct devient sa propre plaque, posée par-dessus les couleurs. Le sens des valeurs de ces canaux n'a pas encore été vérifié sur un TIFF Photoshop de l'atelier : regardez les plaques reprises avant la première impression.

## Comment vérifier avant d'imprimer ?

1. Ouvrez l'aperçu de chaque plaque dans l'inspecteur : le blanc doit couvrir exactement ce que vous voulez.
2. Lisez les alertes du travail, qui signalent les plaques reprises et les tons inconnus.
3. Téléchargez l'**Épreuve TIFF**, à la taille exacte, avec les tons directs gardés en canaux, aplatis dans la couleur (blanc en gris clair, argent en gris) ou supprimés.
4. Imprimez d'abord sur une chute.

Les couleurs métallisées `MT-*` du nuancier Epson ne sont pas encore converties : n'en dépendez pas pour l'instant. Si vous préparez aussi de la découpe, lisez [l'impression-découpe et le repérage](/blog/impression-decoupe-reperage/), et pour la teinte des couleurs elles-mêmes, l'étalonnage et les profils.

## Comment éviter les erreurs courantes ?

- **« Envoi à l'imprimante impossible »** : un ton direct n'a pas d'encre sur la machine. Renommez-le `White` ou `Metallic`, convertissez-le en CMJN, ou ajoutez-le à la table des tons directs avec sa couleur Lab, puis relancez le travail.
- **Blanc visible au bord de l'image** : augmentez la rétraction.
- **Blanc absent** : le nom du ton ne correspond pas, ou l'encre minimale est trop haute pour vos teintes claires.
- **Rien n'est imprimé en connexion Réseau** : voir la note plus haut, passez par une machine en Dossier.

Le guide de dépannage et la [documentation d'utilisation](/documentation/utilisation/) détaillent ces cas.

## À vous de jouer

[Installez E-KO RIP Center gratuitement](/telecharger/) et testez un fichier avec sous-couche blanche. Une question sur votre machine et ses encres spéciales ? [Contactez-nous](/contact/).

## Questions fréquentes

### Comment nommer le blanc dans Illustrator pour le RIP ?

Créez un ton direct nommé `White`. Le RIP reconnaît aussi `Blanc` et `WH`. Pour l'argent, utilisez `Silver` ou `Metallic`.

### Quelle rétraction choisir pour le blanc ?

La valeur par défaut est de 0,2 mm, réglable jusqu'à 5 mm. Commencez par la valeur par défaut, imprimez un essai sur votre support, puis augmentez si du blanc dépasse du bord de la couleur.

### Peut-on imprimer blanc et argent sur la même page ?

Non : la mise en file est refusée avec la raison. Séparez-les en deux pages ou deux travaux.

### Le blanc fonctionne-t-il avec le pilote direct de l'Epson ?

Pas encore. Le blanc et l'argent sont générés et visibles dans les aperçus, mais ils sortent aujourd'hui en fichiers par une machine en connexion Dossier.
