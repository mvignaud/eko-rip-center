---
titre: Set up a RIP workshop in five steps
description: The order to follow to get a workshop running: server, installation, accounts, clients and machines. Allow about an hour for a small workshop.
sous_rubrique: rip-premiers-pas
position: 21
mots_cles: workshop, getting started, installation, order, first production, trial
slug: rip-set-up-a-rip-workshop-in-five-steps
traduction_de: rip-installer-un-atelier-en-cinq-etapes
---

This article gives the order of operations to get a workshop running, from buying the PC to the first print. It points to the detailed article for each step.

## Before you start

Allow about an hour for a simple workshop: one server, one or two workstations, a printer and a cutter. Read [Understand the parts of the RIP](rip-comprendre-les-pieces-du-rip.md) first.

## Steps

1. **Prepare the server.** Choose a 64-bit (x86) PC that will stay on, preferably with an SSD reserved for the data, connected by cable and on a UPS. Give it a fixed IP address on the router. Details: [Choose the server computer](rip-choisir-le-poste-serveur.md).
2. **Install the server.** On Ubuntu 24.04 or 26.04 LTS, add the E-KO repository then install the `eko-rip-serveur` package. The service starts by itself and restarts with the PC. Details: [Install the RIP server](rip-installer-le-serveur.md). For a new PC reserved for the RIP, you can also start from [the installation image](rip-installer-le-serveur-avec-l-image-ubuntu.md).
3. **Create the accounts.** As long as no account exists, any workstation on the network can do everything and the screen flags it in red. Create at least one administrator account, then one operator account per person. Details: [Create the server accounts](../rip-administration/rip-creer-les-comptes.md).
4. **Install the clients.** On each workstation, install the client then choose the server in the **Servers found on the network** list (*Serveurs trouvés sur le réseau*). Details: [Connect to the server from a workstation](rip-se-connecter-au-serveur.md).
5. **Declare the machines.** In the client, **Administration…** (*Administration…*) menu, **System › Machines** (*Système › Machines*) view: add each printer and each cutter, its connection (network, USB, folder) and its media. Details: [Add a machine](../rip-machines/rip-ajouter-une-machine.md).

   ![List of machines in the System view](../../images/interface-systeme-machines.png)
6. Send a small test PDF, check its preview and plates in the inspector, then print it on an offcut. Details: [Send a job](../rip-travaux/rip-envoyer-un-travail.md).

## Good to know

- The server talks to the machines over the network, over USB or through a folder: a network printer needs a fixed IP address and a cable on the same network as the server; a USB cutter needs a free USB port on the server (the service already has the right to write to it); a network cutter needs a fixed IP address; an office printer or another RIP goes through IPP/CUPS or through forwarding to another E-KO RIP Center server. See [Choose how a machine connects](../rip-machines/rip-choisir-la-connexion-d-une-machine.md).
- Every saved preset has a hot folder: a file dropped into it goes to the RIP with that preset, without opening the client. To drop files there from Macs and Windows PCs, see [Share the hot folders](../rip-administration/rip-partager-les-dossiers-surveilles.md).
- Next: install the [mobile app](rip-installer-l-application-mobile.md) to be warned of a breakdown, link the server to the [Cloud](../rip-cloud-premiers-pas/rip-cloud-relier-un-serveur.md) to reach it from outside, and schedule a [configuration backup](../rip-administration/rip-sauvegarder-la-configuration.md).
- The network must let certain ports through: see [Know the server's network ports](rip-connaitre-les-ports-reseau-du-serveur.md).

## See also

- [Understand the parts of the RIP](rip-comprendre-les-pieces-du-rip.md)
- [Choose the server computer](rip-choisir-le-poste-serveur.md)
- [Install the RIP server](rip-installer-le-serveur.md)
- [Add a machine](../rip-machines/rip-ajouter-une-machine.md)
