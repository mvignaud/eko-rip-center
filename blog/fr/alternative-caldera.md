---
titre: Alternative à Caldera : E-KO RIP Center
description: Caldera et E-KO RIP Center tournent tous deux sous Linux. Comparaison honnête : licence, code ouvert, accès distant, machines, et quand préférer Caldera.
date: 2026-11-03
categorie: Comparatifs
image: /assets/img/linux-travaux.webp
alt: Le client Linux d'E-KO RIP Center avec la liste des travaux et l'inspecteur
---
E-KO RIP Center est une alternative libre et gratuite à CalderaRIP pour un atelier grand format dont les machines figurent dans [la liste des machines pilotées](/compatibilite/). Les deux logiciels tournent sous Linux et macOS : la vraie différence est ailleurs, dans le modèle de licence, le code ouvert, l'accès distant sans VPN, et surtout l'étendue du parc de machines, où Caldera est très en avance. Nous éditons E-KO RIP Center : cet article est donc écrit par une partie intéressée. Tous les faits sur Caldera sont sourcés, consultés le 3 novembre 2026.

## Caldera est-il vraiment sous Linux et macOS ?

Oui, et c'est sa force. Caldera se présente comme un RIP « pour macOS et Linux », [pas limité à Windows](https://www.caldera.com/solutions/calderarip-your-rip-software-for-mac-and-linux/). Selon sa [page de prérequis techniques](https://www.caldera.com/resources/technical-requirements/), CalderaRIP V19 est pris en charge sous Ubuntu 22.04 et 24.04 et sous macOS Sonoma 14, Sequoia 15 et Tahoe 26, et Windows n'y figure pas pour le RIP. Les outils périphériques (PrimeCenter, CalderaJobs) existent aussi sous Windows.

E-KO RIP Center suit une logique voisine : [serveur sous Ubuntu ou Debian](/blog/choisir-son-rip-grand-format/), clients natifs Mac, Linux et Windows (installateur Windows pas encore signé). Sur ce point, aucun des deux n'est « l'alternative » de l'autre.

## Quelles différences de licence et de coût ?

Caldera vend CalderaRIP [en abonnement ou en licence perpétuelle](https://www.caldera.com/solutions/calderarip-your-rip-software-for-mac-and-linux/), en plusieurs éditions : Essentials, Pro, Max, Studio, Business (abonnement) et Enterprise (perpétuelle). Chaque édition inclut un pilote d'imprimante et un utilisateur ; le nombre de pilotes supplémentaires dépend de l'édition. Les prix ne sont pas publiés sur la page : voir un revendeur ou demander un devis.

E-KO RIP Center est publié sous [licence AGPL-3.0](/blog/rip-open-source-agpl/) : pas de licence à acheter, pas de nombre de postes ou de machines limité, pas de clé matérielle. Le coût réel d'un atelier est celui du PC serveur et de l'accompagnement que vous choisirez.

## Et le code ouvert ?

CalderaRIP est un logiciel commercial ; la page produit ne mentionne pas de code source public. Celui d'E-KO RIP Center est public sur GitHub. Concrètement : on peut lire comment la séparation des encres ou la découpe sont faites, corriger un défaut ou faire écrire un pilote pour sa machine par un tiers, et le RIP continue de tourner si l'éditeur disparaît.

## Qu'en est-il de l'accès à distance et du suivi de production ?

Caldera fournit CalderaJobs avec le RIP : on y contrôle et libère les travaux, on équilibre la charge entre imprimantes, et on peut l'utiliser [« à distance depuis Windows, macOS ou Linux »](https://www.caldera.com/software-workflow/calderajobs/). Pour relier le RIP à une boutique en ligne, un ERP ou un MIS, Caldera propose [Nexio, par protocoles JDF/JMF](https://www.caldera.com/software-workflow/nexio/), et une [API REST](https://www.caldera.com/solutions/calderarip-your-rip-software-for-mac-and-linux/). Chez E-KO, l'Échange JDF et JMF est En cours… (voir la [feuille de route](/feuille-de-route/)).

Côté E-KO RIP Center, le [Cloud E-KO RIP](/cloud/) relie le serveur de l'atelier à une plate-forme par un tunnel sortant : pas de VPN, pas de port ouvert. Il est gratuit pour 1 imprimante et 1 découpeuse, et il est facultatif. L'application mobile suit la production. Le pont ERP `eko-rip-erp/1` est un contrat public : le RIP appelle l'ERP en HTTPS, et seules des quantités physiques circulent, jamais de montants.

Caldera [appartient au groupe Dover depuis 2017](https://www.caldera.com/about/about-caldera/).

## Combien de machines chaque RIP pilote-t-il ?

C'est l'écart le plus net. Caldera annonce [plus de 1 900 périphériques grand format compatibles](https://www.caldera.com/solutions/calderarip-your-rip-software-for-mac-and-linux/), et existe depuis [1991](https://www.caldera.com/about/about-caldera/). E-KO RIP Center pilote aujourd'hui l'Epson SureColor SC-S80600 (impression), le Roland VersaCAMM VP-300/540 (découpe ; Impression sur Roland VersaCAMM En cours…), le Mutoh Kona 1650 et le Graphtec FC7000 (découpe). Le parc est donc beaucoup plus restreint ; il s'élargit à chaque version (Imprimantes d'autres marques : En cours…), et [la compatibilité est détaillée ici](/compatibilite/) avec un formulaire pour demander votre machine. Certaines machines passent déjà par la connexion Dossier, IPP ou Renvoi.

| | CalderaRIP | E-KO RIP Center |
|---|---|---|
| Serveur | Ubuntu, macOS | Ubuntu, Debian |
| Licence | Abonnement ou perpétuelle, prix non publiés | AGPL-3.0, gratuit |
| Code ouvert | Non annoncé | Oui |
| Parc de machines | Plus de 1 900 périphériques | 4 références pilotées ; Imprimantes d'autres marques : En cours… |
| Accès distant | CalderaJobs à distance | Cloud sans VPN (gratuit pour 1+1 machines) |

## Quand Caldera reste-t-il le meilleur choix ?

- Votre imprimante n'est pas dans la liste d'E-KO : Caldera a de bien plus grandes chances de la piloter.
- Vous avez besoin de ses outils de préparation avancés (imbrication à forme réelle, couleur poussée, [PrimeCenter](https://www.caldera.com/solutions/calderarip-your-rip-software-for-mac-and-linux/)) et d'un support éditeur, avec un réseau de revendeurs.
- Vous préférez un contrat de support commercial à une communauté ouverte.

## Quand essayer E-KO RIP Center ?

Si votre atelier utilise l'une des machines pilotées, que vous voulez zéro licence, un accès distant sans configuration réseau et un code que vous pouvez auditer. L'installation se fait depuis [la page de téléchargement](/telecharger/) et rien n'empêche de tester en parallèle de l'existant. Pour d'autres comparaisons : [Alternative à ONYX](/blog/alternative-onyx/), [Fiery XF ou E-KO RIP Center](/blog/fiery-xf-ou-e-ko-rip/), [Alternative à Flexi (SAi)](/blog/alternative-flexi-sai/) et le [comparatif général](/blog/comparatif-rip-grand-format/). Une question ? [Contactez-nous](/contact/).

*Caldera, CalderaRIP, PrimeCenter, CalderaJobs, Nexio et Dover sont des marques de leurs propriétaires respectifs. Comparatif établi d'après les informations publiques des éditeurs au 3 novembre 2026 ; écrivez-nous pour toute correction.*

## Questions fréquentes

### Caldera fonctionne-t-il sous Linux ?

Oui. D'après le site de Caldera, CalderaRIP V19 est pris en charge sous Ubuntu 22.04 et 24.04, ainsi que sous macOS. E-KO RIP Center fonctionne aussi sous Linux (Ubuntu, Debian) côté serveur.

### E-KO RIP Center est-il vraiment gratuit ?

Oui : le logiciel est libre (AGPL-3.0) et sans licence à payer. Le Cloud E-KO RIP, facultatif, est gratuit pour une imprimante et une découpeuse, puis payant au-delà.

### E-KO RIP Center peut-il piloter mon imprimante ?

Seulement si elle fait partie des modèles pilotés ou passe par Dossier ou IPP. Vérifiez la [page compatibilité](/compatibilite/) ou demandez votre machine ; le parc s'étend à chaque version.

### Peut-on utiliser les deux logiciels ?

Oui. Ils sont indépendants : vous pouvez tester E-KO RIP Center sur une machine de l'atelier sans toucher à votre installation Caldera.
