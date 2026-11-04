---
titre: Set up a job for cutting
description: Rotate the cut, offset it outward, or repeat and scale a cut-only job without rendering again.
sous_rubrique: rip-travaux
position: 57
mots_cles: cut setup, cut rotation, offset, mirror, scale, copies, gap, origin
slug: rip-set-up-a-job-for-cutting
traduction_de: rip-mettre-en-coupe-un-travail
---

The **Cut setup** (*Mise en coupe*) panel rotates the job for the cutter, not the file, with no new render. This article is for operators.

## Before you start

The job has cut paths. Nothing goes out until you save.

## Steps

1. Click the job, then open the inspector's **Cut setup** panel.
2. Set the **Cut rotation** (*Rotation de la coupe*) in quarter turns, registration frame included: turn the media by the same amount. The cut preview carries a grey arrow for the feed direction (the top of the page goes in first).
3. Set **Offset the cut outward (mm)** (*Décaler la coupe vers l'extérieur (mm)*), from 0 to 5 mm: the piece grows by that amount (a hole shrinks).
4. For a printed job whose cut is curved, see [Compensate for print bowing](rip-compenser-le-cintrage.md).
5. For **cut only** jobs only, also set: **Mirror** (*Miroir*), **Scale X / Y (%)** (*Échelle X / Y (%)*; **Proportional**, *Proportionnelle*, or not), **Copies X / Y** with **Gap X / Y (mm)** (*Écart X / Y (mm)*; 100 per direction, 1000 in all) and **Start from the machine origin** (*Départ depuis l'origine de la machine*).
6. Click **Save the cut setup** (*Enregistrer la mise en coupe*): the cut files are rebuilt.

## Good to know

- Mirror, scale and copies are only offered for cut-only jobs: with a print, the cut has no ink to follow.
- The offset does not replace the printer-cutter calibration (the machine's **Offset X / Y**, *Décalage X / Y*): see [Fix a cut that is offset from the print](../rip-depannage/rip-decoupe-decalee.md).
- A rotated or mirrored cut no longer overlays the print in the viewer.

## See also

- [Compensate for print bowing](rip-compenser-le-cintrage.md)
- [Order the cut layers](rip-ordonner-les-couches-de-decoupe.md)
- [Fix a cut that is offset from the print](../rip-depannage/rip-decoupe-decalee.md)
