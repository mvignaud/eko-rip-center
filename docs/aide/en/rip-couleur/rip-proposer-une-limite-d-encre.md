---
titre: Suggest an ink limit from measurements
description: Print solids of increasing ink, measure their L* and get through the API the limit to enter in the mode, along with a CGATS chart.
sous_rubrique: rip-couleur
position: 45
mots_cles: ink limit, linearisation, cgats chart, l*, api, solids, bronzing
slug: rip-suggest-an-ink-limit-from-measurements
traduction_de: rip-proposer-une-limite-d-encre
---

To set the ink limit of a mode, you print solids of increasing ink and measure their lightness. The API suggests the limit. This article is for administrators.

## Before you start

You have a spectrophotometer to measure L*, and an administrator account to call the API.

## Steps

1. Get a linearisation chart: `GET /api/linearisation/mire.cgats?n=21` (option `encres=C,Lc,M` to choose the inks). The CGATS chart gives one patch per ink and per step, tightened in the highlights: 8 patches below 20% out of 21, against 4 for a regular ramp.
2. Print solids of increasing total ink (for example from 100 to 400% in steps of 20%) and measure their L*.
3. Send the series to `POST /api/linearisation/limite-proposee`:

   ```json
   {"mesures": [[100, 58.0], [120, 47.2]]}
   ```

4. Read the response: the suggested limit is the last step that still darkens by at least 1 L* per additional 10% of ink, rounded down to the next 5%; a step that gets lighter (bronzing, bleeding) stops it.
5. Review the gains returned, check the solid once dry, then enter the limit in the mode (see [Set the light inks and total ink of a mode](rip-regler-les-encres-claires-et-l-encre-totale.md)).

## Good to know

- Nothing is saved by the call: you are the one who enters the limit.
- To calibrate grey, see [Calibrate grey (G7-type method)](rip-etalonner-le-gris-type-g7.md).

## See also

- [Set the light inks and total ink of a mode](rip-regler-les-encres-claires-et-l-encre-totale.md)
- [Calibrate grey (G7-type method)](rip-etalonner-le-gris-type-g7.md)
- [Set up a media and its print modes](rip-regler-un-support-et-ses-modes.md)
