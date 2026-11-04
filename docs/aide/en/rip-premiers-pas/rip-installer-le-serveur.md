---
titre: Install the RIP server
description: Install the RIP server on Ubuntu from the E-KO package repository and check that it answers on the workshop network.
sous_rubrique: rip-premiers-pas
position: 2
mots_cles: installation, server, ubuntu, apt, repository, eko-rip-serveur
slug: rip-install-the-rip-server
traduction_de: rip-installer-le-serveur
---

The server installs with two commands on an Ubuntu PC. It starts by itself and restarts with the PC. This article is for the person who administers the server computer.

## Before you start

- A PC chosen as described in [Choose the server computer](rip-choisir-le-poste-serveur.md), running Ubuntu 24.04 or 26.04 LTS (or Debian 13).
- Administrator access (`sudo`) and Internet access on this PC.
- About 10 minutes.

## Steps

1. Open a terminal on the server PC.
2. Add the E-KO repository key:
   `sudo wget -qO /usr/share/keyrings/eko-rip.gpg https://apt.e-ko.fr/rip/eko-rip.gpg`
3. Add the repository:
   `echo "deb [signed-by=/usr/share/keyrings/eko-rip.gpg] https://apt.e-ko.fr/rip ./" | sudo tee /etc/apt/sources.list.d/eko-rip.list`
4. Refresh the package list: `sudo apt update`
5. Install the server: `sudo apt install eko-rip-serveur`
6. Check that the service is running: `systemctl status eko-rip-serveur`
7. From another workstation, open `http://<server-address>:8640` in a browser. The status page should appear.

![Server status page shown in a browser, with the version and the machines](../../images/serveur-page-etat.png)

## Good to know

- The package installs Ghostscript, creates the `eko-rip` system account and stores the jobs in `/var/lib/eko-rip`.
- The browser shows only a status page: this is intended. The full interface is reserved for the clients.
- Without the repository, you can install the `.deb` file attached to each release with `sudo apt install ./<file>.deb`. Updates are then done by hand.
- On Ubuntu 26.04, a server version that is too old fails on every render with "Permission denied": update it.
- As long as no account exists, the whole network has free access. Create the accounts next.

- A PC that serves as both server and workstation can use the client installation script (`wget -qO- https://apt.e-ko.fr/rip/installer | sudo sh`), which adds the same repository: then just run `sudo apt install eko-rip-serveur`.
- Advanced settings (port, data folder, retention…) go in `/etc/default/eko-rip-serveur`: see [Change the server's advanced settings](../rip-administration/rip-modifier-les-reglages-avances-du-serveur.md).
- The ports to let through are described in [Know the server's network ports](rip-connaitre-les-ports-reseau-du-serveur.md).

## See also

- [Create the server accounts](../rip-administration/rip-creer-les-comptes.md)
- [Install the client on Linux](rip-installer-le-client-linux.md)
- [Update the server](../rip-administration/rip-mettre-a-jour-le-serveur.md)
- [Install the server without the repository (package, Docker, binary)](rip-installer-le-serveur-sans-depot.md)
- [Install the server with the Ubuntu installation image](rip-installer-le-serveur-avec-l-image-ubuntu.md)
