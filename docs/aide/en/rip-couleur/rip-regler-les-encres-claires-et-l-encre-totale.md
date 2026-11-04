---
titre: Set the light inks and total ink of a mode
description: Choose the light-ink switch (soft, balanced, sharp) and the total ink limit of an Epson print mode.
sous_rubrique: rip-couleur
position: 44
mots_cles: light-ink switch, total ink, soft, balanced, sharp, mode, lc, lm, lk, ink limit
slug: rip-set-the-light-inks-and-total-ink-of-a-mode
traduction_de: rip-regler-les-encres-claires-et-l-encre-totale
---

Two optional fields of a mode set the separation of the light inks on the Epson (direct driver). This article is for the administrator.

## Before you start

You need an administrator account and a declared media (see [Set up a media and its print modes](rip-regler-un-support-et-ses-modes.md)).

## Steps

1. Open **Administration…** (*Administration…*), **System** (*Système*) view, **Epson media** (*Supports de l'Epson*) section, and the media's **Modes** (*Modes*) field.
2. After the passes, add the light-ink switch and the total ink, in any order: `Affiche = 720 dpi / 8 / franche / 260 %` ("Affiche" is the mode name; "franche" means sharp).
3. Choose the **light-ink switch** (*bascule des claires*), that is, the point where the dark ink takes over from the light one (Lc, Lm, Lk). The values are written in French:

   | Value | Effect |
   |---|---|
   | **douce** (soft) | Light ink full up to 30% density, long handover up to 70%, a little light ink kept under the dark one. The least grain: gradients and skin seen up close. |
   | **équilibrée** (balanced, default) | Light ink alone up to 24%, handover up to 54%. |
   | **franche** (sharp) | Light ink limited to 60%, handover from 18 to 33%. Less ink and faster drying: display seen from afar. |

4. Enter the **total ink** (*Encre totale*; `260 %`, from 100 to 400%): the maximum sum of the 9 inks at one point (300% without it).
5. Click **Save** (*Enregistrer*), then make a test print.

## Good to know

- The switch values are starting values, to be confirmed on paper.
- A third optional field gives the volume of the small, medium and large drops for the estimated ink: `Autre = 360 dpi / 4 / 5-10-15 pL`. Without it, 5-10-15 pL, starting values to be calibrated.
- To find the ink limit, see [Suggest an ink limit from measurements](rip-proposer-une-limite-d-encre.md).

## See also

- [Set up a media and its print modes](rip-regler-un-support-et-ses-modes.md)
- [Suggest an ink limit from measurements](rip-proposer-une-limite-d-encre.md)
- [Understand the Epson direct driver](../rip-machines/rip-comprendre-le-pilote-direct-de-l-epson.md)
