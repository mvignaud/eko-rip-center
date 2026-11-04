---
titre: Fix printing that continues after Stop in the mobile app
description: Why Stop does not halt a print that has already gone to the machine, and how to interrupt it.
sous_rubrique: rip-depannage
position: 73
mots_cles: stop, pause, mobile app, printing in progress, interrupt
slug: rip-fix-printing-continues-after-stop
traduction_de: rip-impression-continue-apres-stop
---

You tapped **Stop** in the app, but the machine keeps printing.

## Before you start

Find the output in the **Queue** tab (*File*) and note its status.

## Steps

1. Check the output's status: **Stop** cancels an output that is **Waiting** (*En attente*) or **Suspended** (*Suspendue*).
2. If the output is **In progress** (*En cours*), it has already gone to the machine: interrupt it on the machine itself.
3. Once the machine has stopped, restart the output if needed with **Retry** (*Relancer*).

## Good to know

- **Pause** and **Stop** only act on an output that has not yet gone to the machine: the server cannot yet interrupt a print in progress.
- An output nested in a sheet has no action in the app.

## See also

- [Act on an output from the mobile app](../rip-travaux/rip-agir-sur-une-sortie-depuis-l-application-mobile.md)
- [Manage a machine's queue](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
- [The output to the machine failed](rip-sortie-vers-la-machine-echouee.md)
