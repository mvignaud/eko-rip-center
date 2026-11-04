---
titre: Choose the server computer
description: Choose the PC, disk and network that will run the RIP server without interruption.
sous_rubrique: rip-premiers-pas
position: 1
mots_cles: server, pc, configuration, disk, ssd, ups, requirements
slug: rip-choose-the-server-computer
traduction_de: rip-choisir-le-poste-serveur
---

The RIP server receives the files, computes the renders and drives the machines. This article helps you choose the PC that will run it, before installation.

## Before you start

Plan for a 64-bit (x86) PC that stays on all the time. Allow a few minutes to choose, plus the time to reserve addresses on your router.

## Steps

1. Choose Ubuntu 26.04 LTS for a production workshop. Ubuntu 24.04 LTS or Debian 13 are fine for trials.
2. Plan for 4 or more cores and 16 to 32 GB of memory for production. For trials, 2 cores and 16 GB are enough.
3. Reserve a disk for the data: a dedicated SSD of 1 TB or more in production, 500 GB for trials. Mount it before the first start.
4. Connect the PC with a network cable, never over Wi-Fi.
5. Plug the server, the switch and, if possible, the internet box into a UPS (uninterruptible power supply).
6. On your router, reserve a fixed address for the server, for each network printer and for each network cutter.

## Good to know

- A render weighs about 1 GB per m² at 600 dpi (a quarter of that at 300 dpi, four times more at 1,200 dpi). A dedicated disk stops a large render from filling the system.
- Below 5 GB free, the server deletes the oldest inactive jobs and refuses a job that would not fit. Below 20 GB, every new job carries the alert "disque presque plein" (disk almost full).
- A power cut stops the render in progress (the job must be retried) and any output being sent (to be redone). The settings files survive the cut.
- More memory and cores shorten the renders of banners of several tens of m² at high resolution.

- The test server in the E-KO workshop runs on a Pentium G645T (2 cores, 2012), 16 GB of memory and a 640 GB mechanical disk with no separate partition: it is enough for trials, but it is a floor, not a target. The figures above are advice, not measurements: the render timeout (`EKO_RIP_DELAI_S`, 30 minutes by default) quickly tells you whether the computer is too weak.
- Allow at least 500 GB, more if you keep jobs for a long time (`EKO_RIP_CONSERVATION_JOURS`). The **Production** (*Production*) view shows the output buffer of the queue and the share of the disk in use. A dedicated disk can be replaced or enlarged without reinstalling; mount it on `/var/lib/eko-rip` (or point `EKO_RIP_DATA` at it).
- Choose a UPS that lasts a few minutes and tells the server to shut down cleanly (for example with `nut`).
- One free USB port is needed for each cutter connected over USB (Kona); the service is in the `dialout` group.
- To compare, see also the [Requirements](https://e-ko.rip/prerequis/) page of the website.

## See also

- [Install the RIP server](rip-installer-le-serveur.md)
- [Connect to the server from a workstation](rip-se-connecter-au-serveur.md)
- [Connect the Epson SureColor SC-S80600](../rip-machines/rip-brancher-epson-sc-s80600.md)
- [Know the server's network ports](rip-connaitre-les-ports-reseau-du-serveur.md)
