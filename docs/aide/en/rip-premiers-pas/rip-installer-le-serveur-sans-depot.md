---
titre: Install the server without the repository (package, Docker, binary)
description: Install the RIP server from the .deb file, a Docker container or the bare binary, when the package repository cannot be used.
sous_rubrique: rip-premiers-pas
position: 24
mots_cles: deb, docker, binary, manual installation, ghostscript, debian, tar.gz
slug: rip-install-the-server-without-the-repository
traduction_de: rip-installer-le-serveur-sans-depot
---

The package repository remains the recommended method. This article describes the three other ways to install the server, for a computer that cannot use the repository.

## Before you start

- A 64-bit **x86 (amd64)** PC: the packages are built for this architecture.
- Administrator access (`sudo`) on the PC.
- The files are downloaded from the [releases page](https://github.com/mvignaud/eko-rip-center/releases).

## Steps

### With the Debian package (.deb)

1. On Debian 13 or Ubuntu 24.04, download `eko-rip-serveur_<version>-1_amd64.deb`.
2. Install it: `sudo apt install ./eko-rip-serveur_<version>-1_amd64.deb`.
3. Check: `systemctl status eko-rip-serveur`, then `journalctl -u eko-rip-serveur -f` to follow the log.

The package installs Ghostscript, creates the `eko-rip` system user and starts the service, which restarts by itself with the machine. Jobs are stored in `/var/lib/eko-rip`; settings go in `/etc/default/eko-rip-serveur`, followed by `sudo systemctl restart eko-rip-serveur`.

### With Docker

1. Sign in to the registry with an access token that has the `read:packages` right: `docker login ghcr.io` (the image repository is private).
2. Start the container:
   `docker run -d --name eko-rip --restart unless-stopped -p 8640:8640 -v eko-rip-donnees:/donnees ghcr.io/mvignaud/eko-rip-center:<version>`

### With the bare binary

1. Download `eko-rip-serveur-<version>-linux-x86_64.tar.gz`, which contains the program.
2. Install Ghostscript: `sudo apt install ghostscript`.
3. Start the program: `./eko-rip-serveur`.

## Good to know

- Installed by hand, the server is not updated with the system: run the same command again with the new file. See [Update the server](../rip-administration/rip-mettre-a-jour-le-serveur.md).
- With the bare binary, settings are given as environment variables (see [Change the server's advanced settings](../rip-administration/rip-modifier-les-reglages-avances-du-serveur.md)); with no settings, the data is stored in the `eko-rip-data` folder, next to the program.
- After installation, open `http://<server-address>:8640` from another workstation: the status page should appear.
- On Ubuntu 26.04, a package version that is too old fails on every render with "Permission denied": use a recent version.

## See also

- [Install the RIP server](rip-installer-le-serveur.md)
- [Install the server with the Ubuntu installation image](rip-installer-le-serveur-avec-l-image-ubuntu.md)
- [Create the server accounts](../rip-administration/rip-creer-les-comptes.md)
