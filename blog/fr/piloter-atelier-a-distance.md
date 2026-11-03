---
titre: Piloter son atelier d'impression à distance, sans VPN
description: Suivez la file, l'encre et les alertes machine de votre atelier depuis un poste ou un téléphone, sans VPN ni port ouvert. Offre gratuite et tarifs du Cloud.
date: 2026-11-03
categorie: Atelier
image: /assets/img/cloud-mobile.webp
alt: Application mobile E-KO RIP et portail Cloud montrant les sites de l'atelier
---
Oui, vous pouvez suivre et lancer les impressions de votre atelier depuis chez vous ou en déplacement, sans VPN et sans ouvrir de port sur la box. Le Cloud E-KO RIP relie le serveur RIP de l'atelier à une plate-forme par une liaison chiffrée que le serveur ouvre lui-même ; le client de bureau et l'application mobile s'y connectent ensuite. Une offre gratuite à vie couvre un site, une imprimante et un découpeur.

## Pourquoi l'accès à distance pose-t-il problème à un atelier ?

Le serveur d'un RIP reste normalement dans le réseau de l'atelier : c'est le choix le plus sûr, mais il vous coupe de votre production dès que vous quittez les lieux. Les solutions classiques demandent un VPN à installer et à maintenir, ou une redirection de port sur la box, ce qui expose un service d'impression à Internet. Pour un atelier sans informaticien, c'est souvent trop lourd, et le suivi se fait alors par téléphone avec l'opérateur.

E-KO RIP Center prend le problème dans l'autre sens : rien ne rentre dans l'atelier, c'est le serveur qui sort.

## Comment fonctionne le Cloud E-KO RIP ?

Le Cloud E-KO RIP est un service facultatif qui donne accès à un serveur RIP depuis l'extérieur. Le serveur ouvre une connexion sortante chiffrée vers la plate-forme ; celle-ci y fait passer les requêtes des postes et de l'application mobile. Rien n'est à régler sur la box ni sur le pare-feu, et chaque site reçoit sa propre adresse.

Le rattachement se fait en trois étapes :

1. **Créer un compte** sur le portail, avec votre e-mail. Aucune carte n'est demandée pour l'offre Gratuite.
2. **Rattacher le serveur** : dans le RIP, rubrique Cloud, demandez un code court, puis saisissez-le sur le portail. Le code est valable dix minutes.
3. **Se connecter de partout** : dans le client ou l'application mobile, « Se connecter au cloud » fait apparaître vos sites.

Prérequis : le serveur doit avoir au moins un compte déclaré. Sans compte, l'accès est libre sur le réseau local et le Cloud refuse de s'ouvrir, par prudence. Après une coupure réseau, le serveur rouvre la liaison tout seul, et vous pouvez la couper depuis le RIP quand vous voulez.

## Que peut-on faire depuis l'application mobile ?

L'application E-KO RIP pour iPhone et Android est un outil de suivi. Elle ne sert pas à envoyer des fichiers ni à régler une impression ou une découpe. Elle montre :

- **la file** de chaque machine, sorties en cours d'abord, avec des actions simples : lancer ou reprendre une sortie suspendue, mettre en pause, annuler une sortie en attente, la passer en tête, relancer une sortie terminée ;
- **les machines** : état, sortie en cours, niveaux d'encre, longueur de rouleau restante et alertes en cours ;
- **les alertes** : les 200 derniers événements.

Une limite à connaître : une impression déjà partie vers la machine ne peut pas être arrêtée depuis l'application, car le serveur ne sait pas encore l'interrompre.

Pour essayer sans rien installer, le mode démo de l'écran de connexion montre l'application avec des données fictives. L'application est annoncée comme bientôt disponible sur l'App Store et Google Play ; la page [application mobile](/app/) donne l'état à jour.

## Quelles alertes machine reçoit-on sur le téléphone ?

Une notification arrive quand un événement demande votre attention :

| Événement | Déclencheur |
|---|---|
| Alerte de la machine | Capot ouvert, support épuisé, etc., lus sur l'imprimante |
| Encre presque vide | Une encre passe à 5 % ou moins |
| Cartouche vide | Une encre atteint 0 % |
| Machine injoignable | Trois relevés manqués de suite, puis « répond de nouveau » |
| Sortie ou rendu en échec | Une impression, une découpe ou un rendu échoue |
| Fin d'impression | Seulement si l'option est cochée dans les réglages |

Les alertes de la machine viennent du serveur RIP, qui les relève chaque minute sur l'Epson par le protocole SNMP. Les textes anglais de la machine sont traduits en français. Après un redémarrage du serveur, aucune rafale de notifications : le premier relevé ne fait que poser le point de départ.

## Combien coûte le Cloud ?

Le RIP lui-même est gratuit, avec ou sans Cloud. Seuls l'accès à distance et la sauvegarde en ligne relèvent des offres ci-dessous, en euros hors taxes par mois. Utilisateurs illimités dans toutes les offres.

| Offre | Prix | Contenu |
|---|---|---|
| Gratuit | 0 € à vie | 1 site, 1 imprimante et 1 découpeur, dernière sauvegarde gardée (1 Go) |
| À la machine | 6 € par machine | 1 site, chaque machine au-delà des machines gratuites, 30 versions de sauvegarde, 5 Go par machine |
| Atelier | 19 € | 1 site, jusqu'à 5 machines, 90 jours d'historique, 25 Go ; machine en plus : 4 € |
| Pro | 49 € | Jusqu'à 3 sites et 15 machines, 1 an d'historique, 100 Go, restauration vers un autre site ; machine en plus : 4 € |
| Entreprise | Sur devis | Sites, machines et stockage sur mesure, accompagnement dédié |

En paiement annuel, deux mois sont offerts. Le paiement se fait par carte ou prélèvement SEPA, avec une facture à chaque échéance. Une première souscription payante bénéficie de 30 jours d'essai. Les offres sont réservées aux professionnels et sans engagement au-delà de la période payée. Le détail est sur la page [Cloud et tarifs](/cloud/).

## Que se passe-t-il si j'arrête le Cloud ?

Le compte revient à l'offre Gratuite, sans frais, et le RIP de l'atelier n'est pas touché : rendu, impression, découpe, files et clients du réseau local ne dépendent pas du Cloud. Au-delà de ce que couvre l'offre gratuite, seul l'accès à distance s'arrête, après quatorze jours de grâce.

## Que contient la sauvegarde en ligne ?

La configuration du serveur : machines, profils couleur, supports et réglages. Elle est faite chaque nuit et après chaque changement, et se restaure depuis le portail. Elle ne contient pas vos fichiers d'impression, qui restent dans l'atelier.

Le guide complet est dans la [documentation du Cloud](/documentation/cloud/). Pour comprendre ce que le serveur fait au quotidien, lisez aussi [qu'est-ce qu'un RIP](/blog/qu-est-ce-qu-un-rip/).

**Essayez gratuitement** : [téléchargez E-KO RIP Center](/telecharger/) puis rattachez votre atelier en quelques minutes. Une question sur l'offre qui vous convient ? [Écrivez-nous](/contact/).

## Questions fréquentes

### Le Cloud est-il obligatoire ?

Non. Le RIP fonctionne entièrement dans l'atelier, sans compte, sans abonnement et sans connexion Internet. Le Cloud ajoute seulement l'accès depuis l'extérieur et la sauvegarde en ligne.

### Faut-il un VPN ou ouvrir un port ?

Non. Le serveur établit lui-même une liaison chiffrée vers la plate-forme, que vous pouvez couper depuis le RIP à tout moment.

### Puis-je inviter mes collègues ?

Oui, en accès complet ou en lecture seule, site par site. Le nombre d'utilisateurs est illimité, quelle que soit l'offre.

### L'application mobile peut-elle envoyer un fichier ?

Non, elle ne fait que du suivi de la file, des machines et des alertes, avec quelques actions sur les sorties.
