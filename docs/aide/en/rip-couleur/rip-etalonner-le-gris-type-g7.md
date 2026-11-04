---
titre: Calibrate grey (G7-type method)
description: Print the grey chart, send the L*a*b* measurements and get the C, M, Y and K correction curves, iteration after iteration.
sous_rubrique: rip-couleur
position: 46
mots_cles: g7, grey, calibration, chart, correction curves, iterations, dot gain, cgats, api
slug: rip-calibrate-grey-g7-type-method
traduction_de: rip-etalonner-le-gris-type-g7
---

Grey calibration corrects the ink curves to obtain a neutral grey. It is driven through the API and is reserved for the administrator. It is not a G7 certification: the method and values are those of the RIP.

## Before you start

You have an administrator account, a spectrophotometer and the calibration media.

## Steps

1. Get the chart: `GET /api/g7/mire.cgats?n=21` gives a CMY ramp with equal parts and a black ramp, from 0 to 100%.
2. Print the chart, then measure it (L*a*b* per patch).
3. Send the measurements to `POST /api/g7/iterations`:

   ```json
   {"nom": "Epson / banner", "mesures": [{"rampe": "cmy", "pct": 50, "l": 58.1, "a": 1.2, "b": -0.4}]}
   ```

   Each ramp needs its paper (0%), its solid (100%) and at least three patches in between. You can add `engraissement` (dot gain; 22% by default at 50%) and `aplats` (the a*b* of the C, M and Y solids).
4. Read the response: the gaps to the target (mean ΔL* and mean grey gap, with `conforme` below 1.5 and 1.5, indicative thresholds), the C, M, Y and K correction curves every 5%, all the iterations of this calibration and the number of the best one.
5. Print the next chart with the curves of the best iteration, and indicate it with `"avec": <its number>` when sending the measurements. The new correction is combined with these curves.

## Good to know

- A worse iteration does not erase the best one; the last 50 are kept.
- Changing `engraissement` or `aplats` recomputes all the iterations on the new target.
- The grey target keeps the paper tint, which fades as the grey gets darker.
- The curves will apply to rendering together with the per-mode linearisation.
- The calibration is included in the configuration backup.

## See also

- [Suggest an ink limit from measurements](rip-proposer-une-limite-d-encre.md)
- [Choose the colour profiles of a job](rip-choisir-les-profils-de-couleur.md)
- [Back up the server configuration](../rip-administration/rip-sauvegarder-la-configuration.md)
