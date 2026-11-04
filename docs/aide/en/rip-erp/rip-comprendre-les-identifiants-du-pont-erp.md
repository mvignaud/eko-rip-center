---
titre: Understand ERP bridge identifiers
description: Machines, media, modes, layers and settings carry an immutable UUID that the ERP must use instead of names.
sous_rubrique: rip-erp
position: 26
mots_cles: uuid, identifier, immutable, machine, media, mode, setting, icc profile, unknown_identifier
slug: rip-understand-erp-bridge-identifiers
traduction_de: rip-comprendre-les-identifiants-du-pont-erp
---

So that the ERP can find its way even when a name changes, each catalogue item carries an identifier assigned by the server. This article explains the rules to follow.

## Before you start

It is for the person who develops or configures the integration with the ERP.

## Steps

1. Rely on the `uuid` (UUID v4) of each **machine**, **cut layer**, **saved setting**, **media** and **print mode**. It is assigned by the server and immutable.
2. Never rely on a name: a machine's displayed name and text identifier (`epson`, `kona`…) can change.
3. Do not invent an identifier: an unknown `uuid` sent to the RIP is refused with the code `identifiant_inconnu` ("unknown identifier").
4. For ICC profiles, use their file name: they have no `uuid`.

## Good to know

- A **renamed** saved setting becomes a different setting, with a new `uuid`: its name is also the name of its watched folder.
- The identifiers are published in the catalogue: see [View the events sent to the ERP](rip-consulter-les-evenements-envoyes-a-l-erp.md).
- A machine's identifier is also the name of the `sorties/<machine>/` folder and is used for forwarding from another RIP: do not change it lightly.

## See also

- [Understand the ERP bridge](rip-comprendre-le-pont-erp.md)
- [View the events sent to the ERP](rip-consulter-les-evenements-envoyes-a-l-erp.md)
- [Add a machine](../rip-machines/rip-ajouter-une-machine.md)
