# Installer le serveur

Le serveur tourne sous Linux. Les imprimantes et les plotters lui sont branchés ;
les postes de l'atelier s'y connectent par le réseau.

Quatre façons de l'installer, au choix.

## 0. PC dédié : image d'installation Ubuntu

> ⚠️ **L'installation efface entièrement le disque du PC.**

L'image est faite à partir d'Ubuntu 24.04 LTS (environ 4 Go). Une release GitHub refuse
les fichiers de plus de 2 Go : elle est publiée en morceaux (`.part0`, `.part1`, `.part2`…)
avec un fichier `.sha256`. Télécharger tous les morceaux et le `.sha256` dans un même
dossier, puis les réunir dans l'ordre :

- Linux ou macOS : `cat eko-rip-center-<version>-ubuntu-24.04-amd64.iso.part* > eko-rip-center-<version>-ubuntu-24.04-amd64.iso`
  puis `shasum -a 256 -c eko-rip-center-<version>-ubuntu-24.04-amd64.iso.sha256` ;
- Windows : `copy /b fichier.iso.part0 + fichier.iso.part1 + fichier.iso.part2 fichier.iso`
  (autant de morceaux que publiés), puis `certutil -hashfile fichier.iso SHA256` à comparer au `.sha256`.

Écrire l'image sur une clé USB (balenaEtcher, ou `dd`), brancher le PC au réseau de
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
