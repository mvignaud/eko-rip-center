---
titre: Understand banded rendering of very large formats
description: Why a very large format is rendered in bands then stitched back together, how much space it needs and why it takes longer.
sous_rubrique: rip-depannage
position: 22
mots_cles: very large format, bands, banner, bigtiff, long render, disk, 3 gb, tiling
slug: rip-understand-banded-rendering
traduction_de: rip-comprendre-le-rendu-par-bandes
---

A very long render is not a fault: the server itself splits very large pages into bands. This article explains the mechanism and the disk space to allow for.

## Before you start

You have a very large format job (for example a banner) whose render takes longer than usual.

## Steps

1. Identify the case: a page whose render would exceed 3 GB, for example a 1.6 × 3 m banner at 720 dpi.
2. Let it run: the server renders it in horizontal bands, then stitches its plates back together. There is nothing to set.
3. Check the free space in the **System** view of the client (**Free space** line, *Espace libre*) before you start.
4. Watch the progress in the queue: the job stays **in progress** (*en cours*) longer.

## Good to know

- A stitched plate larger than 3.5 GB is written as BigTIFF.
- A plate weighs up to width × height in dots, which is nearly 4 GB per ink for this banner, before compression: allow for the space.
- If the render exceeds the maximum time (30 minutes by default), raise `EKO_RIP_DELAI_S`: see [Change the server's advanced settings](../rip-administration/rip-modifier-les-reglages-avances-du-serveur.md).
- Below 5 GB free, the server refuses a job that would not fit: see [Fix a failed render or a rejected file](rip-un-rendu-echoue.md).
- More cores and memory shorten these renders: see [Choose the server computer](../rip-premiers-pas/rip-choisir-le-poste-serveur.md).

## See also

- [Fix a failed render or a rejected file](rip-un-rendu-echoue.md)
- [Choose the server computer](../rip-premiers-pas/rip-choisir-le-poste-serveur.md)
- [Change the server's advanced settings](../rip-administration/rip-modifier-les-reglages-avances-du-serveur.md)
