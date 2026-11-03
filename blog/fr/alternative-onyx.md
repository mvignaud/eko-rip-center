---
titre: Alternative à ONYX RIP : ce qu'il faut savoir
description: ONYX (Thrive, RIPCenter) tourne sous Windows ; E-KO RIP Center a son serveur sous Linux. Licence, accès distant, parc de machines : comparatif sourcé.
date: 2026-11-03
categorie: Comparatifs
image: /assets/img/linux-travaux.webp
alt: Le client Linux d'E-KO RIP Center avec la liste des travaux et l'inspecteur
---
ONYX est l'un des RIP grand format les plus répandus, avec un très large parc de machines ; E-KO RIP Center est une alternative libre et gratuite pour les ateliers qui veulent un serveur Linux, aucune licence et un accès distant sans VPN, à condition que leurs machines soient prises en charge. Nous éditons E-KO RIP Center : lisez ce comparatif avec cela en tête. Les faits sur ONYX viennent de ses pages officielles, consultées le 3 novembre 2026.

## Sous quel système tourne ONYX ?

Sous Windows. ONYX indique que son logiciel « tourne sous Windows » et conseille aux ateliers qui conçoivent sur Mac ou Linux d'y associer [un poste RIP Windows dédié](https://onyxgfx.com/onyx-vs-caldera-rip-software-comparison/). Ses [spécifications système](https://onyxgfx.com/wp-content/uploads/2023/03/ONYX-System-Specs.pdf) citent Windows 10 ou 11 Pro 64 bits, 16 Go de mémoire minimum (32 à 64 Go recommandés) et une clé de sécurité USB. Ce document n'est pas daté : vérifiez la version en vigueur auprès d'ONYX.

E-KO RIP Center fait l'inverse : le serveur tourne sous [Ubuntu ou Debian](/prerequis/), sur un PC 64 bits x86, et les postes utilisent des [clients natifs Mac, Linux et Windows](/telecharger/) (installateur Windows pas encore signé).

## Quelles différences de licence ?

ONYX propose [des licences perpétuelles et un abonnement mensuel (ONYX Go)](https://onyxgfx.com/onyx-vs-caldera-rip-software-comparison/) ; ONYX Advantage est un contrat de mise à jour et de support vendu à part. [ONYX Thrive](https://onyxgfx.com/products/onyx-thrive/) fonctionne avec une licence réseau et permet d'ajouter imprimantes, RIP et découpeuses sans logiciel supplémentaire. [RIPCenter](https://onyxgfx.com/products/rip-products/ripcenter) est présenté comme un RIP pour une seule imprimante. Aucun prix n'est publié : il faut contacter ONYX ou un revendeur.

E-KO RIP Center est sous [licence AGPL-3.0](/blog/rip-open-source-agpl/) : gratuit, sans limite de postes ni de machines, sans clé matérielle ni serveur d'activation.

## Que gagne-t-on, que perd-on en changeant ?

Une migration se juge sur ce que l'on utilise chaque jour, pas sur une liste de fonctions.

ONYX a des atouts réels : un historique depuis [1989](https://onyxgfx.com/products/rip-products/ripcenter), un moteur construit sur Adobe PDF Print Engine, une bibliothèque PANTONE intégrée, une imbrication automatique, du profilage ICC et la gestion des encres spéciales selon les éditions, et un réseau de revendeurs. Il annonce [plus de 2 000 imprimantes et découpeuses compatibles](https://onyxgfx.com/products/onyx-thrive/).

E-KO RIP Center apporte :

- un serveur Linux sans licence, qui reste dans l'atelier ;
- le code public : on peut auditer le rendu, la séparation, la découpe ;
- des clients natifs sur chaque système et une [application mobile](/cloud/) ;
- le blanc et l'argent générés automatiquement, l'imposition, l'imbrication, la découpe avec repérage, le suivi des coûts et un pont ERP public.

Il ne couvre pas tout ce qu'ONYX propose. Parmi ses éditions, ONYX détaille par exemple le profilage ICC, DeviceLink+ ou les outils textile. Côté E-KO, Profils ICC créés à l'atelier et Conversion de couleur entre profils sans perte (DeviceLink) sont En cours… ; pour les outils textile, demandez à votre revendeur ce que vous utilisez réellement. Côté E-KO, l'étalonnage du gris de type G7 (sans certification), les profils ICC d'entrée et de support et la table de tons directs en Lab existent, mais la table des tons directs se remplit à la main : une bibliothèque PANTONE fournie d'office comme chez ONYX s'appelle chez nous Bibliothèques de tons directs fournies : En cours… (voir la [feuille de route](/feuille-de-route/)).

## Et l'accès à distance ?

ONYX vend [ONYX Sync](https://onyxgfx.com/products/onyx-thrive/), un outil séparé de suivi de performance dans le cloud, de liaison entre plusieurs sites et de contrôle à distance du flux de travail. ONYX Hub fournit des analyses de production.

Le [Cloud E-KO RIP](/cloud/) relie le serveur de l'atelier à une plate-forme par un tunnel sortant, sans VPN ni port ouvert. Il est gratuit pour une imprimante et une découpeuse, et facultatif. Le RIP fonctionne entièrement sans lui.

## Combien de machines sont pilotées ?

ONYX annonce plus de 2 000 imprimantes et découpeuses. E-KO RIP Center en pilote nettement moins : l'Epson SureColor SC-S80600 (impression), le Mutoh Kona 1650 et le Graphtec FC7000 (découpe), le Roland VersaCAMM (découpe ; Impression sur Roland VersaCAMM En cours…). Ce parc est plus restreint, Imprimantes d'autres marques est En cours…, il s'élargit à chaque version, et vous pouvez demander votre machine sur la [page compatibilité](/compatibilite/).

| | ONYX | E-KO RIP Center |
|---|---|---|
| Système du serveur | Windows 10/11 Pro | Ubuntu, Debian |
| Licence | Perpétuelle ou abonnement, prix non publiés | AGPL-3.0, gratuit |
| Code ouvert | Non annoncé | Oui |
| Accès distant | ONYX Sync (produit séparé) | Cloud sans VPN, gratuit en base |
| Parc de machines | Plus de 2 000 | 4 références ; Imprimantes d'autres marques : En cours… |

## Quel RIP pour quel atelier ?

- **Votre machine n'est pas pilotée par E-KO** : restez sur ONYX (ou un autre RIP) ; c'est le choix le plus sûr.
- **Vous avez besoin de ses outils avancés** (couleur spectrale, textile, support revendeur) : ONYX reste pertinent.
- **Vous avez un Epson SC-S80600 ou une découpeuse compatible et un PC Linux** : E-KO RIP Center évite la licence et la clé USB.
- **Vous voulez comparer** : installez-le à côté, depuis [la page de téléchargement](/telecharger/). Voir aussi [Alternative à Caldera](/blog/alternative-caldera/), [Fiery XF ou E-KO RIP Center](/blog/fiery-xf-ou-e-ko-rip/), [Alternative à Flexi (SAi)](/blog/alternative-flexi-sai/), le [comparatif général](/blog/comparatif-rip-grand-format/) et [comment choisir son RIP](/blog/choisir-son-rip-grand-format/). Une question ? [Contactez-nous](/contact/).

*ONYX, ONYX Thrive, ONYX RIPCenter, ONYX Go, ONYX Sync, ONYX Hub et PANTONE sont des marques de leurs propriétaires respectifs. Comparatif établi d'après les informations publiques des éditeurs au 3 novembre 2026 ; écrivez-nous pour toute correction.*

## Questions fréquentes

### ONYX fonctionne-t-il sous Linux ?

D'après ses pages officielles, ONYX tourne sous Windows ; aucune prise en charge de Linux n'y est annoncée. E-KO RIP Center a son serveur sous Linux.

### ONYX est-il payant ?

Oui : licences perpétuelles ou abonnement, sans prix publiés. E-KO RIP Center est gratuit sous licence AGPL-3.0.

### Peut-on piloter à distance un RIP E-KO ?

Oui, avec le Cloud E-KO RIP, sans VPN, gratuit pour une imprimante et une découpeuse, ou avec l'application mobile.

### Mon imprimante est-elle compatible avec E-KO RIP Center ?

Consultez la [page compatibilité](/compatibilite/). Si elle n'y figure pas, demandez-la : le parc s'élargit à chaque version.
