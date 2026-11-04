---
titre: Install the server with the Ubuntu installation image
description: Prepare a USB stick with the installation image, choose a profile (server only, with desktop, with client) and install a new PC dedicated to the RIP.
sous_rubrique: rip-premiers-pas
position: 23
mots_cles: image, iso, usb stick, dedicated pc, profile, ubuntu, installation, artcraft, sha256
slug: rip-install-the-server-with-the-ubuntu-installation-image
traduction_de: rip-installer-le-serveur-avec-l-image-ubuntu
---

For a new PC reserved for the RIP, a ready-to-use Ubuntu installation image installs the system, the RIP server and, as you choose, the desktop and the client. It is used only for the first installation: afterwards, the server is updated through the package repository.

## Before you start

> The installation completely erases the PC's disk.

- A 64-bit (x86) PC chosen as described in [Choose the server computer](rip-choisir-le-poste-serveur.md), connected to the workshop network by a cable (the Ubuntu desktop is downloaded during installation).
- A USB stick of at least 8 GB. The image weighs about 4 GB. It is built from Ubuntu 24.04 LTS.
- The image and its fingerprint file (`.sha256`), available from the [image download folder](https://nuage.e-ko.fr/s/5fekYTc2pzXRor6).
- About 30 to 45 minutes.

## Steps

1. Download the image (a single `.iso` file) and its `.sha256` file, into the same folder.
2. Check the file's fingerprint:
   - on Linux or macOS: `shasum -a 256 -c <file>.iso.sha256`;
   - on Windows: `certutil -hashfile <file>.iso SHA256`, or, in PowerShell, `(Get-FileHash <file>.iso -Algorithm SHA256).Hash`, then compare with the contents of the `.sha256` file.
3. Write the image to the USB stick with balenaEtcher, Rufus or `dd`. With Rufus, choose **DD Image mode** and not ISO mode: ISO mode rebuilds the boot and loses the profile menu.
4. Plug the stick into the PC, connect the PC to the workshop network and boot from the stick.
5. In the menu, choose a profile:

   | Profile | For | Contents |
   |---|---|---|
   | **Serveur + client sur ce PC** (*Server + client on this PC*, default) | small workshop, a single PC | Ubuntu desktop, RIP server, E-KO RIP Center client opened at every session on this PC |
   | **Serveur + bureau Ubuntu** (*Server + Ubuntu desktop*) | everyday PC that is also the server, clients on other workstations | Ubuntu desktop, RIP server |
   | **Serveur seul (sans bureau)** (*Server only, no desktop*) | PC dedicated to the RIP | RIP server, administration over SSH |
6. Fill in the requested identity: your name, the PC name (`eko-rip` recommended), the account name (`eko` recommended) and its password.
7. Confirm the disk erasure: at the question "Continue with autoinstall?", answer **yes**.
8. Remove the stick when the PC restarts.
9. Check the start-up: the PC starts in French, with the server running. On the "server + client" profile, the client opens and connects to the local server by itself.
10. From the other workstations, connect the client to the address shown on the connection screen (see [Connect to the server from a workstation](rip-se-connecter-au-serveur.md)). To administer the server-only profile, use `ssh eko@<server-address>`.

## Good to know

- The two profiles with a desktop add **CUPS**, to print to office and network printers (set up in *Paramètres › Imprimantes*, i.e. Settings › Printers), and the ArtCraft suite: free editing applications (PhotoCraft, VectorCraft, PdfCraft, DesignCraft…, thirteen in all).
- The ArtCraft suite is downloaded during installation (about 1 GB) from its official releases, with the SHA-256 fingerprint of each package checked. If the download fails, the PC installation carries on; the details are in `/var/log/eko-artcraft.log`. To install or update later: `sudo eko-artcraft` (the whole suite) or `sudo eko-artcraft vectorcraft pdfcraft`.
- Newer versions of the server are then installed by updating the package: see [Update the server](../rip-administration/rip-mettre-a-jour-le-serveur.md).
- Once installed, create the accounts: [Create the server accounts](../rip-administration/rip-creer-les-comptes.md).

## See also

- [Choose the server computer](rip-choisir-le-poste-serveur.md)
- [Install the RIP server](rip-installer-le-serveur.md)
- [Install the server without the repository (package, Docker, binary)](rip-installer-le-serveur-sans-depot.md)
- [Update the server](../rip-administration/rip-mettre-a-jour-le-serveur.md)
