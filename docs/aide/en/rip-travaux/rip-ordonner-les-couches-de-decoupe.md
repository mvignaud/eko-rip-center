---
titre: Order the cut layers
description: Set the cutting order, turn each layer on or off and choose its cut setting: kiss cut, perforation, creasing and through cut.
sous_rubrique: rip-travaux
position: 54
mots_cles: cut layers, cutting order, cut setting, creasing, perforation, v-cut, one layer at a time
slug: rip-order-the-cut-layers
traduction_de: rip-ordonner-les-couches-de-decoupe
---

Each cut spot colour in the file becomes a layer, cut in the order you choose. This article is for cutter operators.

## Before you start

The job has cut paths (see [Set cut prefixes](rip-regler-les-prefixes-de-decoupe.md)). The cutter has at least one cut setting (see [Set up the Kona cut](../rip-machines/rip-regler-la-coupe-du-kona.md)).

## Steps

1. Click the job, then open the inspector's **Cut layers** (*Couches de découpe*) panel: one layer per spot colour name, with its number of paths.
2. Set the order with ↑ and ↓. **The layer order is the cutting order**: kiss cut before through cut.
3. Tick or untick a layer's box to cut it or not.
4. Choose its **Cut setting** (*Réglage de coupe*). By default, **— same name, otherwise default —** (*— même nom, sinon défaut —*): the cutter's setting that has the layer's name applies by itself, otherwise the machine's first one. You can force another one.
5. Click **Save the layers** (*Enregistrer les couches*): the cut files are rebuilt. **Cancel** (*Annuler*) goes back to the saved layers.

## Good to know

- Starting order: creasing (`Creasing`, `Score`), perforation and V-cut first, the cut last, while the material still holds.
- Nothing is kept without **Save the layers**. Saved layers persist across restarts and the next render.
- A job from before layers keeps its old cut setting: it goes to the layer of the same name or to the only layer; otherwise the panel flags it, to be chosen layer by layer.
- Within a layer, an outline contained in another (the hole of an "O", a window) is cut before it, so the piece does not move; at equal depth, the blade goes to the nearest outline.
- On the machine, **One layer at a time** (*Une couche à la fois*; System > Machines): for a cutter that changes neither tool nor setting during a cut, each layer becomes an output, the next one held until **Resume**.
- A layer with no setting on the chosen cutter makes it unavailable: printing waits.

## See also

- [Set up the Kona cut](../rip-machines/rip-regler-la-coupe-du-kona.md)
- [Set up a job for cutting](rip-mettre-en-coupe-un-travail.md)
- [Choose the cutter for a job](rip-choisir-la-decoupeuse-d-un-travail.md)
