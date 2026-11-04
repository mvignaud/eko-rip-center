---
titre: Add a machine to the RIP
description: Declare a printer or a cutter in the server's machine catalogue, with its connection and settings.
sous_rubrique: rip-machines
position: 1
mots_cles: machine, catalogue, add, model, roll width, connection, offset, throughput
slug: rip-add-a-machine
traduction_de: rip-ajouter-une-machine
---

The RIP only drives the machines declared in its catalogue. This article is for the administrator.

## Before you start

You need an administrator account. The machine is connected (see its model's page) and, if it is on the network, it has a fixed address reserved on the router.

## Steps

1. In the client, open **Administration…**, **System › Machines** view (*Système › Machines*).
2. Click **Add a machine** (*Ajouter une machine*).
3. Choose the **model**. The **Help** link (*Aide*) next to it opens that machine's page.
4. Give an identifier (lowercase letters, digits and `-`) and a name.
5. Enter the roll width.
6. Choose the connection and fill in its fields (see [Choose a machine's connection](rip-choisir-la-connexion-d-une-machine.md)).
7. If needed, set the cut's **X / Y offset** (*Décalage X / Y*), the **Throughput (m²/h)** (*Cadence (m²/h)*) and the **Pool**.
8. Click **Save** (*Enregistrer*): the machine immediately appears in the print buttons, the queue and the sidebar.

![List of machines in Administration, System view, with the server's machine catalogue](../../images/interface-systeme-machines.png)

## Good to know

- The identifier names the `sorties/<machine>/` folder and is used for resubmission from another RIP: do not change it lightly.
- A machine that still has an output to run cannot be removed: let the output through or cancel it first.
- The **X / Y offset** (−20 to 20 mm) corrects the gap measured on a test cut between the cut and the print.
- With the same **Pool** on identical machines (same model, same roll width), a **Print on pool** button (*Imprimer sur pool*) appears on every page.
- For a cutter, **Duplicate (other set of marks)** (*Dupliquer (autre jeu de repères)*) creates a variant of the same physical machine.
- Every field of the machine page is described in [Understand a machine's fields](rip-comprendre-les-champs-d-une-machine.md).
- Supported models: Epson SC-S80600, Roland VersaCAMM VP-300 / VP-540, Mutoh Kona 1650, Graphtec FC7000. For another model, write to contact@e-ko.rip with the make, the model and how it is connected.

## See also

- [Choose a machine's connection](rip-choisir-la-connexion-d-une-machine.md)
- [Know the supported machines](rip-connaitre-les-machines-prises-en-charge.md)
- [Fix a cut that is offset from the print](../rip-depannage/rip-decoupe-decalee.md)
