# Installer le serveur

Le serveur tourne sous Linux. Les imprimantes et les plotters lui sont branchés ;
les postes de l'atelier s'y connectent par le réseau.

Quatre façons de l'installer, au choix.

## 0. PC dédié : image d'installation

> ⚠️ **L'installation efface entièrement le disque du PC.**

Pour un PC x86_64 qui ne servira qu'au RIP. Télécharger
`eko-rip-center-serveur-<version>-amd64.iso` depuis la page des releases et l'écrire
sur une clé USB (balenaEtcher, ou `dd` sous Linux et macOS).

1. Brancher le PC au réseau de l'atelier (câble conseillé : l'installation télécharge
   les paquets Debian).
2. Démarrer sur la clé et choisir **Installer E-KO RIP - EFFACE LE DISQUE**.
3. Répondre aux deux seules questions : le mot de passe du compte `eko` (deux fois),
   puis **Oui** à « Faut-il appliquer les changements sur les disques ? ».
4. Retirer la clé quand l'installation redémarre la machine.

Le PC démarre sous Debian 13, en français, avec le serveur actif. L'écran de
connexion affiche l'adresse à ouvrir depuis les postes, par exemple
`http://192.168.1.20:8640`. L'administration se fait par SSH :
`ssh eko@<adresse>`.

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

`utilisateur liste` liste les comptes ; `utilisateur supprimer <nom>` en retire un (ses
sessions tombent aussitôt). Aucun redémarrage n'est nécessaire. Les mots de passe sont
gardés hachés en Argon2id dans `<données>/utilisateurs.json` (droits 0600).

Une fois connecté, la session dure 12 heures (cookie `HttpOnly`, `SameSite=Strict`).
Elle est perdue au redémarrage du serveur. Après 5 essais faux pour un même nom, ce nom
est bloqué 5 minutes. Les dossiers surveillés ne demandent pas de compte : leur accès
se règle par le partage (Samba).

### Réseau

La connexion n'est pas chiffrée (HTTP) : le serveur ne doit être joignable que depuis
le réseau de l'atelier. Ne jamais ouvrir le port 8640 sur Internet.
