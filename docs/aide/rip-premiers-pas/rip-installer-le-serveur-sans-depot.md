---
titre: Installer le serveur sans le dépôt (paquet, Docker, binaire)
description: Installez le serveur RIP à partir du fichier .deb, d'un conteneur Docker ou du binaire seul, quand le dépôt de paquets n'est pas utilisable.
sous_rubrique: rip-premiers-pas
position: 24
mots_cles: deb, docker, binaire, installation manuelle, ghostscript, debian, tar.gz
---

La méthode conseillée reste le dépôt de paquets. Cet article décrit les trois autres façons d'installer le serveur, pour un poste qui ne peut pas utiliser le dépôt.

## Avant de commencer

- Un PC 64 bits **x86 (amd64)** : les paquets sont construits pour cette architecture.
- Un accès administrateur (`sudo`) sur le PC.
- Les fichiers se téléchargent depuis la [page des versions sur GitHub](https://github.com/mvignaud/eko-rip-center/releases).

## Étapes

### Avec le paquet Debian (.deb)

1. Sur Debian 13 ou Ubuntu 24.04, téléchargez `eko-rip-serveur_<version>-1_amd64.deb`.
2. Installez-le : `sudo apt install ./eko-rip-serveur_<version>-1_amd64.deb`.
3. Vérifiez : `systemctl status eko-rip-serveur`, puis `journalctl -u eko-rip-serveur -f` pour suivre le journal.

Le paquet installe Ghostscript, crée l'utilisateur système `eko-rip` et démarre le service, qui redémarre seul avec la machine. Les travaux sont rangés dans `/var/lib/eko-rip` ; les réglages se font dans `/etc/default/eko-rip-serveur`, suivis de `sudo systemctl restart eko-rip-serveur`.

### Avec Docker

1. Connectez-vous au registre avec un jeton GitHub qui a le droit `read:packages` : `docker login ghcr.io` (le dépôt d'images est privé).
2. Lancez le conteneur :
   `docker run -d --name eko-rip --restart unless-stopped -p 8640:8640 -v eko-rip-donnees:/donnees ghcr.io/mvignaud/eko-rip-center:<version>`

### Avec le binaire seul

1. Téléchargez `eko-rip-serveur-<version>-linux-x86_64.tar.gz`, qui contient le programme.
2. Installez Ghostscript : `sudo apt install ghostscript`.
3. Lancez le programme : `./eko-rip-serveur`.

## Bon à savoir

- Installé à la main, le serveur ne se met pas à jour avec le système : refaites la même commande avec le nouveau fichier. Voir [Mettre à jour le serveur](../rip-administration/rip-mettre-a-jour-le-serveur.md).
- Avec le binaire seul, les réglages se donnent par variables d'environnement (voir [Modifier les réglages avancés du serveur](../rip-administration/rip-modifier-les-reglages-avances-du-serveur.md)) ; sans réglage, les données sont rangées dans le dossier `eko-rip-data`, à côté du programme.
- Après l'installation, ouvrez `http://<adresse-du-serveur>:8640` depuis un autre poste : la page d'état doit s'afficher.
- Sous Ubuntu 26.04, une version trop ancienne du paquet échoue sur chaque rendu avec « Permission denied » : prenez une version récente.

## Voir aussi

- [Installer le serveur RIP](rip-installer-le-serveur.md)
- [Installer le serveur avec l'image d'installation Ubuntu](rip-installer-le-serveur-avec-l-image-ubuntu.md)
- [Créer les comptes du serveur](../rip-administration/rip-creer-les-comptes.md)
