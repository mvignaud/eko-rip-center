---
titre: Installer un atelier RIP en cinq étapes
description: L'ordre à suivre pour mettre en route un atelier : serveur, installation, comptes, clients et machines. Comptez environ une heure pour un petit atelier.
sous_rubrique: rip-premiers-pas
position: 21
mots_cles: atelier, mise en route, installation, ordre, première production, essai
---

Cet article donne l'ordre des opérations pour mettre en route un atelier, de l'achat du PC à la première impression. Il renvoie vers l'article détaillé de chaque étape.

## Avant de commencer

Prévoyez environ une heure pour un atelier simple : un serveur, un ou deux postes, une imprimante et une découpeuse. Lisez d'abord [Comprendre les pièces du RIP](rip-comprendre-les-pieces-du-rip.md).

## Étapes

1. **Préparez le serveur.** Choisissez un PC 64 bits (x86) qui restera allumé, de préférence avec un SSD réservé aux données, branché en câble et sur onduleur. Donnez-lui une adresse IP fixe sur le routeur. Détail : [Choisir le poste serveur](rip-choisir-le-poste-serveur.md).
2. **Installez le serveur.** Sur Ubuntu 24.04 ou 26.04 LTS, ajoutez le dépôt E-KO puis installez le paquet `eko-rip-serveur`. Le service démarre seul et redémarre avec le PC. Détail : [Installer le serveur RIP](rip-installer-le-serveur.md). Pour un PC neuf réservé au RIP, vous pouvez aussi partir de [l'image d'installation](rip-installer-le-serveur-avec-l-image-ubuntu.md).
3. **Créez les comptes.** Tant qu'aucun compte n'existe, n'importe quel poste du réseau peut tout faire et l'écran le signale en rouge. Créez au moins un compte administrateur, puis un compte opérateur par personne. Détail : [Créer les comptes du serveur](../rip-administration/rip-creer-les-comptes.md).
4. **Installez les clients.** Sur chaque poste, installez le client puis choisissez le serveur dans la liste **Serveurs trouvés sur le réseau**. Détail : [Se connecter au serveur depuis un poste](rip-se-connecter-au-serveur.md).
5. **Déclarez les machines.** Dans le client, menu **Administration…**, vue **Système › Machines** : ajoutez chaque imprimante et chaque découpeuse, sa connexion (réseau, USB, dossier) et ses supports. Détail : [Ajouter une machine](../rip-machines/rip-ajouter-une-machine.md).

   ![Liste des machines dans la vue Système](../images/interface-systeme-machines.png)
6. Envoyez un petit PDF d'essai, vérifiez son aperçu et ses plaques dans l'inspecteur, puis imprimez-le sur une chute. Détail : [Envoyer un travail](../rip-travaux/rip-envoyer-un-travail.md).

## Bon à savoir

- Le serveur parle aux machines par le réseau, en USB ou par un dossier : une imprimante en réseau a besoin d'une adresse IP fixe et d'un câble sur le même réseau que le serveur ; une découpeuse en USB, d'un port USB libre sur le serveur (le service a déjà le droit d'y écrire) ; une découpeuse en réseau, d'une adresse IP fixe ; une imprimante de bureau ou un autre RIP passent par IPP/CUPS ou par un renvoi vers un autre serveur E-KO RIP Center. Voir [Choisir la connexion d'une machine](../rip-machines/rip-choisir-la-connexion-d-une-machine.md).
- Chaque réglage enregistré a un dossier surveillé : un fichier déposé dedans part au RIP avec ce réglage, sans ouvrir le client. Pour y déposer depuis les Mac et les PC Windows, voir [Partager les dossiers surveillés](../rip-administration/rip-partager-les-dossiers-surveilles.md).
- Ensuite : installez l'[application mobile](rip-installer-l-application-mobile.md) pour être prévenu d'une panne, rattachez le serveur au [Cloud](../rip-cloud-premiers-pas/rip-cloud-relier-un-serveur.md) pour y accéder de l'extérieur, et programmez une [sauvegarde de la configuration](../rip-administration/rip-sauvegarder-la-configuration.md).
- Le réseau doit laisser passer certains ports : voir [Connaître les ports réseau du serveur](rip-connaitre-les-ports-reseau-du-serveur.md).

## Voir aussi

- [Comprendre les pièces du RIP](rip-comprendre-les-pieces-du-rip.md)
- [Choisir le poste serveur](rip-choisir-le-poste-serveur.md)
- [Installer le serveur RIP](rip-installer-le-serveur.md)
- [Ajouter une machine](../rip-machines/rip-ajouter-une-machine.md)
