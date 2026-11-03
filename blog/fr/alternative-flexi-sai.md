---
titre: Alternative à Flexi (SAi) : E-KO RIP Center
description: Flexi (SAi) réunit conception, RIP et découpe sous Windows, par abonnement. E-KO RIP Center est libre, sur serveur Linux. Comparatif sourcé du 3 novembre 2026.
date: 2026-11-03
categorie: Comparatifs
image: /assets/img/linux-travaux.webp
alt: Le client Linux d'E-KO RIP Center avec la liste des travaux et l'inspecteur
---
Flexi, édité par SAi, est un logiciel tout-en-un d'enseigne et d'impression-découpe (conception, RIP, découpe) sous Windows, vendu par abonnement, avec un parc de machines très étendu. E-KO RIP Center est un RIP seul, libre et gratuit, avec un serveur Linux : une alternative pour un atelier dont les machines sont pilotées par E-KO, pas un remplaçant de la partie conception de Flexi. Nous éditons E-KO RIP Center ; les faits sur SAi sont sourcés, consultés le 3 novembre 2026.

## Qu'est-ce que Flexi, exactement ?

Flexi est une famille de logiciels de SAi. [Flexi Complete](https://www.thinksai.com/latam/flexi-complete/) est présenté comme un ensemble de conception, RIP, impression-découpe et découpe directe de vinyle, avec un RIP 64 bits (« Production Manager »), un moteur de découpe de contour, de l'imbrication à forme réelle, du tuilage, de la gestion de couleur (remplacement de couleur, tons directs, profils ICC) et un outil d'approbation de maquette. [Flexi DESIGN](https://www.thinksai.com/products-flexi/) n'inclut pas de RIP : il envoie à un Production Manager distant ou à un autre RIP.

E-KO RIP Center ne fait pas de conception graphique : il reçoit vos PDF (et AI compatible PDF, SVG, TIFF, PNG, JPEG) et s'occupe du rendu, de l'imposition, de l'impression et de la découpe. Si vous utilisez Flexi pour dessiner, il se remplace en partie seulement : la création graphique y est hors périmètre.

## Sous quel système tourne Flexi ?

Windows 10 ou plus récent. Une version macOS existe, [mais seulement pour Flexi DESIGN](https://www.thinksai.com/products-flexi/), sans le RIP : un Mac [envoie à un ordinateur Windows qui exécute Production Manager](https://support.thinksai.com/hc/en-us/articles/4415528518420-FAQ-for-FlexiDESIGN-for-MacOS). Selon les [exigences de Flexi 21](https://support.thinksai.com/hc/en-us/articles/11461954425364-Flexi-21-System-Requirements), Linux, Android, iOS et ARM ne sont pas pris en charge.

E-KO RIP Center a son [serveur sous Ubuntu ou Debian](/prerequis/) et des clients natifs [Mac, Linux et Windows](/telecharger/) (installateur Windows pas encore signé).

## Licence et prix : que publie SAi ?

SAi vend par abonnement et affiche ses tarifs : Flexi Complete [à partir d'environ 92 $ par mois selon la page de l'éditeur pour l'Amérique latine](https://www.thinksai.com/latam/flexi-complete/) (consultée le 3 novembre 2026). L'abonnement inclut pilotes, mises à jour et support téléphonique et e-mail, et permet de piloter jusqu'à 5 imprimantes ou découpeuses en même temps. Les essais de 7 jours se transforment en abonnement s'ils ne sont pas annulés.

E-KO RIP Center est sous [AGPL-3.0](/blog/rip-open-source-agpl/) : gratuit, sans limite de machines, sans abonnement. Le Cloud est facultatif, gratuit pour une imprimante et une découpeuse.

## Et l'accès à distance ?

SAi décrit son logiciel comme le « premier RIP compatible cloud » ; une connexion internet est requise pour l'activation de la licence et les fonctions cloud, d'après les [exigences](https://support.thinksai.com/hc/en-us/articles/11461954425364-Flexi-21-System-Requirements). Les pages consultées ne détaillent pas le pilotage à distance d'un atelier.

Le [Cloud E-KO RIP](/cloud/) relie le serveur à une plate-forme par un tunnel sortant, sans VPN ni port ouvert, avec client Mac, Linux, Windows et application mobile. E-KO fonctionne sans connexion Internet dans l'atelier : seule la mise à jour et le Cloud en ont besoin.

## Combien de machines sont pilotées ?

SAi annonce [plus de 7 500 imprimantes et plus de 2 200 découpeuses](https://www.thinksai.com/latam/flexi-complete/). C'est de loin le plus grand parc de cette série d'articles. E-KO RIP Center est très en retrait : Epson SureColor SC-S80600, Mutoh Kona 1650, Graphtec FC7000, Roland VersaCAMM (découpe ; Impression sur Roland VersaCAMM En cours…). Imprimantes d'autres marques est aussi En cours… (voir la [feuille de route](/feuille-de-route/)). Le parc s'élargit à chaque version ; [demandez votre machine](/compatibilite/).

| | Flexi (SAi) | E-KO RIP Center |
|---|---|---|
| Système | Windows ; macOS pour DESIGN seulement | Serveur Ubuntu/Debian ; clients natifs Mac, Linux, Windows (installateur Windows pas encore signé) |
| Licence | Abonnement, tarif publié | AGPL-3.0, gratuit |
| Conception graphique | Oui (Flexi Complete) | Hors périmètre |
| Code ouvert | Non annoncé | Oui |
| Parc de machines | Plus de 7 500 imprimantes, 2 200 découpeuses | 4 références ; Imprimantes d'autres marques : En cours… |

## Que fait E-KO que Flexi ne met pas en avant ?

D'après les pages consultées, Flexi est pensé pour un poste Windows qui pilote jusqu'à 5 machines. E-KO RIP Center sépare le calcul des postes : un serveur Linux fait le rendu, plusieurs opérateurs partagent la même file, et l'on peut fermer le client sans interrompre un travail. Il propose aussi le blanc et l'argent générés automatiquement, un journal de consommation d'encre et de support avec export CSV, des dossiers surveillés par réglage et un pont ERP public. Ces fonctions ne sont pas nécessairement absentes de Flexi ; les pages consultées ne les détaillent simplement pas.

## Quand Flexi reste-t-il le meilleur choix ?

- Vous dessinez, imprimez et découpez avec un seul outil, par exemple en enseigne ou en covering.
- Votre machine est absente de la liste d'E-KO : le parc de Flexi est sans commune mesure.
- Vous voulez un support téléphonique et un abonnement tout compris.

## Quand regarder E-KO RIP Center ?

Si vous faites déjà votre conception ailleurs (Illustrator, Affinity), que votre atelier a un PC Linux et l'une des machines pilotées, et que vous voulez supprimer l'abonnement. Essai : [téléchargement](/telecharger/). Autres comparatifs : [Caldera](/blog/alternative-caldera/), [ONYX](/blog/alternative-onyx/), [Fiery XF](/blog/fiery-xf-ou-e-ko-rip/), le [comparatif général](/blog/comparatif-rip-grand-format/) et [choisir son RIP](/blog/choisir-son-rip-grand-format/). Une question ? [Contactez-nous](/contact/).

*Flexi, FlexiPRINT, FlexiDESIGN et SAi sont des marques de leurs propriétaires respectifs. Comparatif établi d'après les informations publiques des éditeurs au 3 novembre 2026 ; écrivez-nous pour toute correction.*

## Questions fréquentes

### Flexi fonctionne-t-il sous Mac ou Linux ?

Selon SAi, seul Flexi DESIGN (sans RIP) existe sous macOS ; le RIP tourne sous Windows. Linux n'est pas pris en charge.

### Flexi est-il gratuit ?

Non : abonnement mensuel ou annuel à tarif publié. E-KO RIP Center est gratuit (AGPL-3.0).

### E-KO RIP Center remplace-t-il Flexi ?

Pour le RIP, l'impression et la découpe de ses machines, oui ; pour la conception graphique, non.

### Mon traceur est-il compatible avec E-KO RIP Center ?

Voir la [page compatibilité](/compatibilite/) ; sinon, demandez-le.
