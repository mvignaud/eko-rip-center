---
titre: Set open paths, sorting and after-cut
description: Allow open paths, choose the cutting order of paths and decide what happens to a finished cut (history, archive or deletion).
sous_rubrique: rip-travaux
position: 60
mots_cles: open paths, banner vents, path sorting, after cut, archive, restore, cut on arrival
slug: rip-set-open-paths-sorting-and-after-cut
traduction_de: rip-regler-les-traces-ouverts-le-tri-et-l-apres-coupe
---

Three settings in the **Cutting** section and one machine setting decide how paths are cut and what happens to the finished cut. This article is for the administrator.

## Before you start

You need an administrator account. The **Cutting** (*Découpe*) block is in the **System** view.

## Steps

1. Open **Administration…**, **System** view, **Cutting** block.
2. Tick or untick **Allow open paths (banner vents)** (*Autoriser les tracés ouverts (évents de bâche)*; ticked by default). Unticked, an open path is not cut, with one alert per page.
3. Tick or untick **Sort paths** (*Trier les tracés*; ticked by default): inner ones first, then nearest neighbour. Unticked, paths are cut in their creation order.
4. Choose **After cut** (*Après coupe*): **do nothing (history)** (*ne rien faire (historique)*; default), **archive (restorable)** (*archiver (restaurable)*) or **remove from the queue** (*supprimer de la file*).
5. Click **Save** (*Enregistrer*).
6. For each cut to arrive on hold, open **System > Machines**, the cutter's record, and untick **Cut on arrival** (*Couper dès l'arrivée*; ticked by default). Each cut then arrives held and goes when the operator clicks **Resume**.

## Good to know

- An archived cut leaves the queue and goes to its machine's **Archive**; **Restore** puts it back in the queue (held on a machine without cut on arrival), **Remove** deletes it.
- Also tick **Also cut paths on non-printable layers** in this block to cut paths on non-printable layers.
- These settings apply to later renders.

## See also

- [Set cut prefixes](rip-regler-les-prefixes-de-decoupe.md)
- [Manage a machine's queue](rip-gerer-la-file-d-une-machine.md)
- [Order the cut layers](rip-ordonner-les-couches-de-decoupe.md)
