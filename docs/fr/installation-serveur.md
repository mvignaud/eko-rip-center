# Installer le serveur

Le serveur tourne sous Linux. Les imprimantes et les plotters lui sont branchés ;
les postes de l'atelier s'y connectent par le réseau.

Trois façons de l'installer, au choix.

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

## Sécurité

Le serveur n'a pas encore d'authentification. Il ne doit être joignable que depuis
le réseau de l'atelier : ne jamais ouvrir le port 8640 sur Internet.
