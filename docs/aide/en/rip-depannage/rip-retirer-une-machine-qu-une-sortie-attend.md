---
titre: Remove a machine that an output is still waiting for
description: A machine cannot be removed from the catalogue while an output is waiting for it: let the output through or cancel it first.
sous_rubrique: rip-depannage
position: 23
mots_cles: remove machine, delete machine, output waiting, catalogue, restore, queue
slug: rip-remove-machine-with-waiting-output
traduction_de: rip-retirer-une-machine-qu-une-sortie-attend
---

The message "Une sortie attend encore cette machine dans la file." ("An output is still waiting for this machine in the queue.") appears when you remove a machine from the catalogue while an output still targets it.

## Before you start

You are an administrator. Decide whether the waiting outputs should go through or be dropped.

## Steps

1. Open the **Production** view and find, in the machine's queue, the outputs still waiting.
2. Let them go through, or cancel them.
3. Open **Administration…**, the **System** view (*Système*), the **Machines** section.
4. Remove the machine again.

## Good to know

- The same check applies to a backup restore: a machine cannot disappear from the catalogue while an output is waiting for it.
- A machine's identifier names the `sorties/<machine>/` folder and is used for resubmission from another RIP: do not change it lightly.

## See also

- [Add a machine](../rip-machines/rip-ajouter-une-machine.md)
- [Manage a machine's queue](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
- [Rebuild a server from a backup](../rip-administration/rip-remonter-un-serveur.md)
