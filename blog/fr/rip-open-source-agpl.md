---
titre: Pourquoi un RIP open source sous licence AGPL-3.0 ?
description: Un RIP libre sous AGPL-3.0, qu'est-ce que cela change pour un atelier ? Pérennité, transparence, pas de clé matérielle ni d'activation, contribution.
date: 2026-11-03
categorie: Logiciel
image: /assets/img/linux-travaux.webp
alt: Client Linux E-KO RIP Center affichant la liste des travaux et leur statut
---
Un RIP open source sous AGPL-3.0 est un logiciel dont le code est public, que vous pouvez utiliser, étudier, modifier et redistribuer, à condition que toute version modifiée diffusée ou proposée en ligne reste ouverte sous la même licence. Pour un atelier, cela veut dire une production qui ne dépend ni d'un abonnement, ni d'une clé matérielle, ni d'un serveur d'activation. E-KO RIP Center est publié ainsi.

## Qu'est-ce que la licence AGPL-3.0 ?

L'AGPL-3.0 (GNU Affero General Public License, version 3) est une licence libre dite « copyleft », conçue pour les logiciels qui tournent sur un serveur. E-KO RIP Center est publié sous la version 3.0 ou ultérieure. Concrètement :

- **utiliser** le logiciel, pour n'importe quel usage, y compris professionnel et commercial ;
- **étudier** son fonctionnement, car le code source est lisible ;
- **modifier** le code pour l'adapter à vos besoins ;
- **redistribuer** le logiciel, modifié ou non.

La contrepartie est la réciprocité : une version modifiée qui est distribuée, ou proposée à d'autres par le réseau, doit publier ses sources sous la même licence. C'est la particularité de l'AGPL par rapport à la GPL, qui ne couvre pas l'usage par le réseau. Les noms et logos E-KO ne sont pas couverts par la licence du code. Et Ghostscript, que le serveur lance comme programme séparé, garde sa propre licence.

Cette licence n'oblige en rien un atelier qui utilise le logiciel tel quel. Elle concerne ceux qui le modifient puis le diffusent.

## Que change un RIP libre pour la pérennité de l'atelier ?

Un RIP est l'outil qui fait tourner vos machines. S'il s'arrête, la production s'arrête. Avec E-KO RIP Center, il n'y a pas de licence à renouveler, pas de clé matérielle et pas de serveur d'activation : le RIP s'installe et fonctionne entièrement dans l'atelier, sans compte, sans abonnement et sans connexion Internet. Rendu, impression, découpe, files d'attente et clients du réseau local n'en dépendent pas.

Si l'éditeur disparaissait, le logiciel continuerait de tourner, et n'importe qui pourrait reprendre le code. C'est la garantie la plus concrète du modèle libre : votre outil de production n'a pas de point de défaillance commercial.

Le Cloud E-KO RIP, qui apporte l'accès à distance et la sauvegarde en ligne, est un service facultatif. Vous pouvez le quitter à tout moment, le RIP continue comme avant. Les offres sont décrites dans [l'article sur le pilotage à distance](/blog/piloter-atelier-a-distance/).

## Faut-il payer une licence par poste ?

Non. Le RIP est gratuit, et le dépôt public contient le serveur, les clients Mac, Linux et Windows, le Cloud et la documentation. Il n'y a pas de licence par poste ni de dongle à brancher. Le nombre de postes qui se connectent au serveur de l'atelier n'est pas un critère de facturation, puisqu'il n'y a pas de facturation du logiciel.

Seul le Cloud a des tarifs, pour l'accès à distance et la sauvegarde ; l'offre gratuite couvre un site, une imprimante et un découpeur, et les utilisateurs sont illimités dans toutes les offres.

## Que peut-on vérifier quand le code est ouvert ?

Le rendu, la séparation des encres, le tramage et la découpe sont lisibles. Quand une couleur sort mal, on peut aller voir pourquoi au lieu d'attendre une réponse du support. C'est aussi vrai de ce qui concerne vos données.

| Question | Ce que le code permet de vérifier |
|---|---|
| Où vont mes fichiers ? | Tout se passe sur le serveur de l'atelier |
| Que remonte le logiciel ? | Les statistiques d'installation sont anonymes, détaillées et désactivables |
| Comment est fait le rendu ? | Le rendu passe par Ghostscript, lancé en mode isolé |
| Comment l'ERP est-il relié ? | Le contrat du pont `eko-rip-erp/1` est public |

Un contrat public signifie que n'importe quel ERP ou outil maison peut se brancher au RIP. L'[article sur le lien RIP et ERP](/blog/relier-rip-et-erp/) détaille ce qui circule.

## Comment un atelier peut-il contribuer ?

Il n'est pas nécessaire de savoir programmer. Les contributions utiles sont de plusieurs ordres :

- **signaler un problème** : les problèmes signalés depuis le logiciel deviennent des tickets publics ;
- **demander un pilote de machine** : le formulaire de la page [compatibilité machines](/compatibilite/) indique marque, modèle et branchement ; une capture du flux envoyé par le logiciel actuel de la machine accélère beaucoup le travail ;
- **proposer une correction ou un pilote** : le dépôt public accepte les propositions, et le guide de contribution du dépôt explique la marche à suivre ;
- **améliorer la documentation**, publiée en français et en anglais.

Chaque atelier a ses machines. Un pilote ouvert, d'autres peuvent l'étendre à leur imprimante ou à leur découpeuse, et tout le monde en profite. C'est ainsi que la liste des machines prises en charge s'allonge.

## Open source veut-il dire sans support ?

Non. Libre ne signifie ni abandonné ni sans interlocuteur : le logiciel est édité par GOLDEN PRINT, qui publie une version presque chaque semaine avec un journal des changements en français et en anglais. Pour une question, l'adresse [contact](/contact/) est faite pour cela. Le code ouvert ajoute une garantie : si le support disparaît, vous gardez l'outil.

Pour comprendre ce que fait concrètement ce logiciel, lisez [qu'est-ce qu'un RIP](/blog/qu-est-ce-qu-un-rip/), ou consultez la [compatibilité des machines](/compatibilite/) avant de l'installer.

**Essayez gratuitement** : [téléchargez E-KO RIP Center](/telecharger/), installez-le sur un PC de l'atelier et jugez sur pièces. Une question sur la licence ou une contribution ? [Écrivez-nous](/contact/).

## Questions fréquentes

### Puis-je utiliser E-KO RIP Center dans un atelier commercial ?

Oui. L'AGPL-3.0 autorise tous les usages, y compris professionnels. Elle n'impose de publier les sources que si vous diffusez une version modifiée ou la proposez à d'autres par le réseau.

### Y a-t-il un dongle ou une clé d'activation ?

Non. Pas de clé matérielle, pas de licence à renouveler, pas de serveur d'activation. Le RIP fonctionne sans connexion Internet.

### Que se passe-t-il si l'éditeur s'arrête ?

Le logiciel continue de tourner dans votre atelier, et le code public peut être repris par d'autres.

### Le Cloud est-il aussi libre ?

Le code du Cloud est dans le même dépôt public. Son usage est un service facultatif, avec une offre gratuite et des offres payantes.
