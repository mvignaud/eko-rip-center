---
titre: Installer le serveur RIP
description: Installez le serveur RIP sous Ubuntu par le dépôt de paquets E-KO et vérifiez qu'il répond sur le réseau de l'atelier.
sous_rubrique: rip-premiers-pas
position: 2
mots_cles: installation, serveur, ubuntu, apt, dépôt, eko-rip-serveur
---

Le serveur s'installe en deux commandes sur un PC Ubuntu. Il démarre seul et redémarre avec le PC. Cet article s'adresse à la personne qui administre le poste serveur.

## Avant de commencer

- Un PC choisi comme décrit dans [Choisir le poste serveur](rip-choisir-le-poste-serveur.md), sous Ubuntu 24.04 ou 26.04 LTS (ou Debian 13).
- Un accès administrateur (`sudo`) et un accès à Internet sur ce PC.
- Environ 10 minutes.

## Étapes

1. Ouvrez un terminal sur le PC serveur.
2. Ajoutez la clé du dépôt E-KO :
   `sudo wget -qO /usr/share/keyrings/eko-rip.gpg https://apt.e-ko.fr/rip/eko-rip.gpg`
3. Ajoutez le dépôt :
   `echo "deb [signed-by=/usr/share/keyrings/eko-rip.gpg] https://apt.e-ko.fr/rip ./" | sudo tee /etc/apt/sources.list.d/eko-rip.list`
4. Actualisez la liste des paquets : `sudo apt update`
5. Installez le serveur : `sudo apt install eko-rip-serveur`
6. Vérifiez que le service tourne : `systemctl status eko-rip-serveur`
7. Depuis un autre poste, ouvrez `http://<adresse-du-serveur>:8640` dans un navigateur. La page d'état doit s'afficher.

![Page d'état du serveur affichée dans un navigateur, avec la version et les machines](../images/serveur-page-etat.png)

## Bon à savoir

- Le paquet installe Ghostscript, crée le compte système `eko-rip` et range les travaux dans `/var/lib/eko-rip`.
- Le navigateur n'affiche qu'une page d'état : c'est voulu. L'interface complète est réservée aux clients.
- Sans dépôt, vous pouvez installer le fichier `.deb` joint à chaque version sur GitHub avec `sudo apt install ./<fichier>.deb`. Les mises à jour sont alors à faire à la main.
- Sous Ubuntu 26.04, une version trop ancienne du serveur échoue sur chaque rendu avec « Permission denied » : mettez-la à jour.
- Tant qu'aucun compte n'existe, tout le réseau a un accès libre. Créez ensuite les comptes.

## Voir aussi

- [Créer les comptes du serveur](../rip-administration/rip-creer-les-comptes.md)
- [Installer le client sur Linux](rip-installer-le-client-linux.md)
- [Mettre à jour le serveur](../rip-administration/rip-mettre-a-jour-le-serveur.md)
- Page détaillée : https://e-ko.rip/documentation/serveur/
