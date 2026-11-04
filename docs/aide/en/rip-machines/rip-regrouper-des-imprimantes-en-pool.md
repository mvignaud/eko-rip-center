---
titre: Group printers into a pool
description: Give identical machines the same pool name so that each output goes to the first machine that is ready.
sous_rubrique: rip-machines
position: 42
mots_cles: pool, identical printers, print on pool, first machine ready, loaded media
slug: rip-group-printers-into-pool
traduction_de: rip-regrouper-des-imprimantes-en-pool
---

A pool spreads prints across several identical machines. This article is for the administrator.

## Before you start

The machines have the same model and the same roll width, and load the same media.

## Steps

1. Open **Administration…**, the **System** view (*Système*), the **Machines** section.
2. In the page of each machine in the group, enter the same name in **Pool**.
3. Click **Save** (*Enregistrer*).
4. On each page of a job, click the **Print on pool "…"** button (*Imprimer sur pool « … »*).
5. Follow the output: it shows on every member and goes to the first machine that is ready, which takes it.

## Good to know

- Queueing is refused if the known loaded media of the members differ: load the same media, or duplicate the machine.
- A proof print cannot be made on a machine in the same pool as the requested print.
- The **Pool** field is empty ("none") by default.

## See also

- [Understand a machine's fields](rip-comprendre-les-champs-d-une-machine.md)
- [Manage a machine's queue](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
- [Approve a proof with the client](../rip-travaux/rip-valider-un-bat.md)
