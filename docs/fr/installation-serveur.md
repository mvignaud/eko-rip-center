# Installer le serveur

Le serveur tourne sous Linux. Les imprimantes et les plotters lui sont branchés ;
les postes de l'atelier s'y connectent par le réseau.

Quatre façons de l'installer, au choix. Ensuite : [administrer le serveur](administration.md)
(données, sauvegardes, mises à jour, questions fréquentes).

## 0. PC dédié : image d'installation Ubuntu

> ⚠️ **L'installation efface entièrement le disque du PC.**

L'image est faite à partir d'Ubuntu 24.04 LTS (environ 4 Go). Elle se télécharge en un
seul fichier, avec son empreinte SHA-256, depuis le nuage d'E-KO :

| Version | Image | Empreinte | Dossier |
|---|---|---|---|
| 0.16.0 | [eko-rip-center-0.16.0-ubuntu-24.04-amd64.iso](https://nuage.e-ko.fr/public.php/dav/files/5fekYTc2pzXRor6/eko-rip-center-0.16.0-ubuntu-24.04-amd64.iso) | [.sha256](https://nuage.e-ko.fr/public.php/dav/files/5fekYTc2pzXRor6/eko-rip-center-0.16.0-ubuntu-24.04-amd64.iso.sha256) | [lien](https://nuage.e-ko.fr/s/5fekYTc2pzXRor6) |

Les versions plus récentes du serveur s'installent ensuite par mise à jour du paquet (voir
[l'administration](administration.md)) : l'image ne sert qu'à la première installation.
Vérifier le fichier téléchargé :

- Linux ou macOS : `shasum -a 256 -c eko-rip-center-0.16.0-ubuntu-24.04-amd64.iso.sha256` ;
- Windows : `certutil -hashfile eko-rip-center-0.16.0-ubuntu-24.04-amd64.iso SHA256`, ou dans PowerShell
  `(Get-FileHash eko-rip-center-0.16.0-ubuntu-24.04-amd64.iso -Algorithm SHA256).Hash`, à comparer au `.sha256`.

Écrire l'image sur une clé USB (balenaEtcher, Rufus en **mode Image DD** — pas le mode ISO,
qui refait le démarrage et perd le menu des profils —, ou `dd`), brancher le PC au réseau de
l'atelier (câble conseillé : le bureau Ubuntu se télécharge pendant l'installation) et
démarrer sur la clé. Le menu propose trois profils :

| Profil | Pour | Contenu |
|---|---|---|
| **Serveur + client sur ce PC** (par défaut) | petit atelier, un seul PC | bureau Ubuntu, serveur RIP, client E-KO RIP Center ouvert à chaque session sur ce PC |
| **Serveur + bureau Ubuntu** | PC d'usage qui fait aussi serveur, clients sur d'autres postes | bureau Ubuntu, serveur RIP |
| **Serveur seul (sans bureau)** | PC dédié au RIP | serveur RIP, administration par SSH |

Les deux profils avec bureau ajoutent :
- **CUPS**, pour imprimer sur les imprimantes de bureau et du réseau (réglage dans
  *Paramètres > Imprimantes*) ;
- la suite **ArtCraft** (applications libres d'édition : PhotoCraft, VectorCraft,
  PdfCraft, DesignCraft…, treize en tout), téléchargée pendant l'installation (≈ 1 Go)
  depuis ses releases officielles `github.com/storytold/<application>`, empreinte SHA-256
  de chaque paquet vérifiée. Si le téléchargement échoue, l'installation du PC continue ;
  le détail est dans `/var/log/eko-artcraft.log`. Pour installer ou mettre à jour plus
  tard : `sudo eko-artcraft` (toute la suite) ou `sudo eko-artcraft vectorcraft pdfcraft`.

L'installateur demande seulement l'identité (votre nom, le nom du PC — `eko-rip`
conseillé —, le nom du compte — `eko` conseillé — et son mot de passe), puis confirme
l'effacement du disque (« Continue with autoinstall? » : répondre
**yes**). Retirer la clé au redémarrage.

Le PC démarre en français, serveur actif. Sur le profil « serveur + client », le client
s'ouvre et se connecte tout seul au serveur local. Depuis les autres postes, le client
se connecte à l'adresse affichée à l'écran de connexion (par exemple
`http://192.168.1.20:8640`). Administration : `ssh eko@<adresse>`.

## 1. Paquet Debian (recommandé)

Pour Debian 13 (trixie) ou Ubuntu 24.04, sur une machine x86_64. Télécharger
`eko-rip-serveur_<version>_amd64.deb` depuis la page des releases, puis :

```bash
sudo apt install ./eko-rip-serveur_0.1.0-1_amd64.deb
```

Le paquet installe Ghostscript, crée l'utilisateur système `eko-rip` et démarre le
service. Il redémarre seul avec la machine.

```bash
systemctl status eko-rip-serveur
journalctl -u eko-rip-serveur -f
```

Les travaux sont rangés dans `/var/lib/eko-rip`. Les réglages se font dans
`/etc/default/eko-rip-serveur`, suivis de `sudo systemctl restart eko-rip-serveur`.

## 2. Docker

```bash
docker run -d --name eko-rip --restart unless-stopped \
  -p 8640:8640 -v eko-rip-donnees:/donnees \
  ghcr.io/mvignaud/eko-rip-center:0.1.0
```

Le dépôt est privé : `docker login ghcr.io` avec un jeton GitHub qui a le droit
`read:packages`.

## 3. Binaire seul

`eko-rip-serveur-<version>-linux-x86_64.tar.gz` contient le programme. Il faut
installer Ghostscript (`apt install ghostscript`), puis lancer `./eko-rip-serveur`.

## Réglages

| Variable | Défaut | Rôle |
|---|---|---|
| `EKO_RIP_ADDR` | `0.0.0.0:8640` | adresse et port d'écoute |
| `EKO_RIP_DATA` | `eko-rip-data` (paquet : `/var/lib/eko-rip`) | dossier des travaux |
| `EKO_RIP_GS` | `gs` | commande Ghostscript |
| `EKO_RIP_MAX_MO` | `2048` | taille maximale d'un PDF reçu, en Mo |
| `EKO_RIP_DELAI_S` | `1800` | durée maximale d'un rendu, en secondes |
| `EKO_RIP_CONSERVATION_JOURS` | `30` | travaux effacés après ce nombre de jours (0 : jamais) |
| `EKO_RIP_SEUIL_LIBRE_MO` | `5120` | espace libre à garder (Mo) : en dessous, les plus vieux travaux inactifs sont purgés et un travail qui ne tiendrait pas est refusé (`espace_insuffisant`) ; 0 : aucun seuil |
| `EKO_RIP_PLOTTER` | `/dev/kona` | port du plotter de découpe |
| `EKO_RIP_PLOTTER_STTY` | `9600 cs8 -cstopb -parenb crtscts raw -echo` | réglages du port du plotter |
| `EKO_RIP_EPSON_SNMP` | (vide) | adresse IP de l'Epson : largeur du média chargé lue par SNMP |
| `EKO_RIP_EPSON_COMMUNAUTE` | `public` | communauté SNMP de l'Epson |
| `EKO_RIP_ACCES_NAVIGATEUR` | `0` | `1` : interface complète aussi dans un navigateur (développement seulement) |

## Partager les dossiers surveillés (Samba)

Pour déposer des fichiers depuis les postes Mac et Windows :

```bash
sudo apt install samba
```

Ajouter à la fin de `/etc/samba/smb.conf` :

```ini
[eko-rip]
   path = /var/lib/eko-rip/entrees
   writable = yes
   force user = eko-rip
   valid users = eko
```

Puis `sudo smbpasswd -a eko` et `sudo systemctl restart smbd`. Les postes ouvrent
`smb://<serveur>/eko-rip` (Mac) ou `\\<serveur>\eko-rip` (Windows).

## Sécurité

### Comptes

Sans compte déclaré, l'accès reste libre et l'écran l'affiche en rouge. Pour fermer
l'accès, créer au moins un compte sur le serveur (le mot de passe est demandé deux
fois, sans écho, 10 caractères au moins) :

```bash
sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur utilisateur ajouter mathieu
```

Deux rôles :
- **administrateur** : tout, y compris les réglages du serveur (catalogue des machines,
  tarifs d'encre et de support, profils ICC, réglages enregistrés) ;
- **opérateur** : envoie, édite, imprime, découpe, gère la file et les planches ; voit
  les machines et le média chargé, mais pas les écrans de réglage, que le serveur lui
  refuse de toute façon.

Le rôle se donne en troisième argument (`admin` ou `operateur`). Sans lui : administrateur
s'il n'y en a encore aucun, opérateur sinon. Les comptes créés avant la 0.11 restent
administrateurs. Redéclarer un compte change son rôle et son mot de passe.

```bash
sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur utilisateur ajouter julie operateur
```

`utilisateur liste` liste les comptes et leur rôle ; `utilisateur supprimer <nom>` en retire un (ses
sessions tombent aussitôt). Aucun redémarrage n'est nécessaire. Les mots de passe sont
gardés hachés en Argon2id dans `<données>/utilisateurs.json` (droits 0600).

Une fois connecté, la session dure 12 heures (cookie `HttpOnly`, `SameSite=Strict`).
Elle est perdue au redémarrage du serveur. Après 5 essais faux pour un même nom, ce nom
est bloqué 5 minutes. Les dossiers surveillés ne demandent pas de compte : leur accès
se règle par le partage (Samba).

### Sauvegarde de la configuration

Système > **Sauvegarde de la configuration** (administrateur) : **Télécharger la
sauvegarde** donne un fichier JSON avec le catalogue des machines, les tarifs, les
réglages enregistrés et les profils ICC déposés. **Restaurer…** remplace machines, tarifs
et réglages par ceux du fichier et ajoute ses profils ; tout est vérifié avant la
première écriture (un fichier faux ne change rien). Les comptes n'y sont pas (les
recréer sur le serveur), ni les travaux ni la file. Pour un serveur neuf : installer,
créer le compte administrateur, restaurer.

### Réseau

La connexion n'est pas chiffrée (HTTP) : le serveur ne doit être joignable que depuis
le réseau de l'atelier. Ne jamais ouvrir le port 8640 sur Internet.
