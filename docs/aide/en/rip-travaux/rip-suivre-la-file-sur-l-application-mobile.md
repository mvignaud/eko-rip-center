---
titre: Follow the print queue on the mobile app
description: Read the E-KO RIP app's Queue tab: each machine's outputs, states, durations and error messages.
sous_rubrique: rip-travaux
position: 70
mots_cles: mobile app, queue, outputs, states, failure, estimated duration
slug: rip-follow-the-queue-on-the-mobile-app
traduction_de: rip-suivre-la-file-sur-l-application-mobile
---

The app's **Queue** (*File*) tab lists each machine's outputs. It lets you see at a glance what is printing, what is waiting and what has failed.

## Before you start

The app is connected to the server (see [Connect the mobile app to the workshop server](../rip-premiers-pas/rip-connecter-l-application-mobile-a-l-atelier.md)).

## Steps

1. Open the **Queue** tab.
2. Find the machine: its outputs are grouped under its name. Outputs in progress and waiting come first, then recently finished ones.
3. Read each output's state: **In progress** (*En cours*), **Waiting** (*En attente*), **Held** (*Suspendue*), **Finished** (*Terminée*), **Failed** (*Échec*) or **Cancelled** (*Annulée*).
4. Read the elapsed time and the estimated time, when known.
5. For a failed output, read the error message shown under it.

![Queue tab: in-progress and waiting outputs of each machine](../../images/mobile-file.png)

## Good to know

- A held output shows the reason for the hold (for example a media change to make).
- A machine with no output shows "Aucune sortie en cours ou en attente" (no output in progress or waiting).
- To act on an output, see [Act on an output from the mobile app](rip-agir-sur-une-sortie-depuis-l-application-mobile.md).
- To send a file, use a workstation: the app sends no files.

## See also

- [Act on an output from the mobile app](rip-agir-sur-une-sortie-depuis-l-application-mobile.md)
- [Follow machine status on the mobile app](rip-suivre-les-machines-sur-l-application-mobile.md)
- [Manage a machine's queue](rip-gerer-la-file-d-une-machine.md)
