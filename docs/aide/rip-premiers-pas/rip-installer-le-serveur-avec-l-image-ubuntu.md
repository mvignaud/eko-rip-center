---
titre: Installer le serveur avec l'image d'installation Ubuntu
description: Préparez une clé USB avec l'image d'installation, choisissez un profil (serveur seul, avec bureau, avec client) et installez un PC neuf dédié au RIP.
sous_rubrique: rip-premiers-pas
position: 23
mots_cles: image, iso, clé usb, pc dédié, profil, ubuntu, installation, artcraft, sha256
---

Pour un PC neuf réservé au RIP, une image d'installation Ubuntu prête à l'emploi installe le système, le serveur RIP et, au choix, le bureau et le client. Elle ne sert qu'à la première installation : ensuite, le serveur se met à jour par le dépôt de paquets.

## Avant de commencer

> L'installation efface entièrement le disque du PC.

- Un PC 64 bits (x86) choisi comme décrit dans [Choisir le poste serveur](rip-choisir-le-poste-serveur.md), relié au réseau de l'atelier par un câble (le bureau Ubuntu se télécharge pendant l'installation).
- Une clé USB d'au moins 8 Go. L'image pèse environ 4 Go. Elle est faite à partir d'Ubuntu 24.04 LTS.
- L'image et son fichier d'empreinte (`.sha256`), téléchargeables dans le [dossier de téléchargement des images](https://nuage.e-ko.fr/s/5fekYTc2pzXRor6).
- Environ 30 à 45 minutes.

## Étapes

1. Téléchargez l'image (un seul fichier `.iso`) et son fichier `.sha256`, dans le même dossier.
2. Vérifiez l'empreinte du fichier :
   - sous Linux ou macOS : `shasum -a 256 -c <fichier>.iso.sha256` ;
   - sous Windows : `certutil -hashfile <fichier>.iso SHA256`, ou, dans PowerShell, `(Get-FileHash <fichier>.iso -Algorithm SHA256).Hash`, puis comparez au contenu du fichier `.sha256`.
3. Écrivez l'image sur la clé USB avec balenaEtcher, Rufus ou `dd`. Avec Rufus, choisissez le **mode Image DD** et non le mode ISO : celui-ci refait le démarrage et perd le menu des profils.
4. Branchez la clé sur le PC, branchez le PC au réseau de l'atelier et démarrez sur la clé.
5. Dans le menu, choisissez un profil :

   | Profil | Pour | Contenu |
   |---|---|---|
   | **Serveur + client sur ce PC** (par défaut) | petit atelier, un seul PC | bureau Ubuntu, serveur RIP, client E-KO RIP Center ouvert à chaque session sur ce PC |
   | **Serveur + bureau Ubuntu** | PC d'usage qui fait aussi serveur, clients sur d'autres postes | bureau Ubuntu, serveur RIP |
   | **Serveur seul (sans bureau)** | PC dédié au RIP | serveur RIP, administration par SSH |
6. Renseignez l'identité demandée : votre nom, le nom du PC (`eko-rip` conseillé), le nom du compte (`eko` conseillé) et son mot de passe.
7. Confirmez l'effacement du disque : à la question « Continue with autoinstall? », répondez **yes**.
8. Retirez la clé au redémarrage.
9. Vérifiez le démarrage : le PC démarre en français, serveur actif. Sur le profil « serveur + client », le client s'ouvre et se connecte tout seul au serveur local.
10. Depuis les autres postes, connectez le client à l'adresse affichée à l'écran de connexion (voir [Se connecter au serveur depuis un poste](rip-se-connecter-au-serveur.md)). Pour administrer le serveur seul, utilisez `ssh eko@<adresse-du-serveur>`.

## Bon à savoir

- Les deux profils avec bureau ajoutent **CUPS**, pour imprimer sur les imprimantes de bureau et du réseau (réglage dans *Paramètres › Imprimantes*), et la suite ArtCraft : applications libres d'édition (PhotoCraft, VectorCraft, PdfCraft, DesignCraft…, treize en tout).
- La suite ArtCraft est téléchargée pendant l'installation (environ 1 Go) depuis ses versions officielles, avec vérification de l'empreinte SHA-256 de chaque paquet. Si le téléchargement échoue, l'installation du PC continue ; le détail est dans `/var/log/eko-artcraft.log`. Pour installer ou mettre à jour plus tard : `sudo eko-artcraft` (toute la suite) ou `sudo eko-artcraft vectorcraft pdfcraft`.
- Les versions plus récentes du serveur s'installent ensuite par mise à jour du paquet : voir [Mettre à jour le serveur](../rip-administration/rip-mettre-a-jour-le-serveur.md).
- Une fois installé, créez les comptes : [Créer les comptes du serveur](../rip-administration/rip-creer-les-comptes.md).

## Voir aussi

- [Choisir le poste serveur](rip-choisir-le-poste-serveur.md)
- [Installer le serveur RIP](rip-installer-le-serveur.md)
- [Installer le serveur sans le dépôt (paquet, Docker, binaire)](rip-installer-le-serveur-sans-depot.md)
- [Mettre à jour le serveur](../rip-administration/rip-mettre-a-jour-le-serveur.md)
