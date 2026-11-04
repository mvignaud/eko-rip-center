---
titre: Build a sheet by hand or with variable data
description: Place each copy to the millimetre, use a cell template or print one value per CSV row, through the sheets API.
sous_rubrique: rip-travaux
position: 67
mots_cles: manual sheet, template, variable data, csv, api, snapping, grid, lock
slug: rip-build-a-sheet-by-hand-or-with-variable-data
traduction_de: rip-composer-une-planche-par-l-api
---

The manual placement screen does not exist yet in the native clients: you drive it through the server API. This article is for integrators and administrators.

## Before you start

You know how to call the server API with an authorised account. Templates are stored by name with `GET`/`POST /api/gabarits` (administrator).

## Steps

1. Choose the placement mode: only one of `motif`, `manuel` and `gabarit`, in the body of `POST /api/planches` (or `POST /api/planches/apercu` for the preview).
2. For **manual** placement, send `manuel`:

   ```json
   { "poses": [{ "copie": 0, "x_mm": 0, "y_mm": 0, "angle": 0, "verrou": true }],
     "grille_mm": 10, "magnetisme_mm": 5 }
   ```

   Copies are counted in the order of the items, starting at 0; x and y give the bottom-left corner on the sheet.
3. For a **template**, send `{ "nom", "largeur_mm", "hauteur_mm", "colonnes", "rangees", "espace_mm" }`: one copy per cell, left to right then bottom to top, centred and laid down if it only fits rotated.
4. For **variable data** (on a single page), send `donnees_variables`:

   ```json
   { "csv": "Nom;Numéro\n…", "champs": [{ "colonne": "Nom", "x_mm": 10, "y_mm": 5, "taille_pt": 12 }] }
   ```

5. Check the preview, then create the sheet.

## Good to know

- A **locked** copy is taken as is. The others are snapped to the **grid**, then **magnetised** (within `magnetisme_mm`) to the edge of the roll or the edge of a copy already placed, to within the spacing.
- A copy that leaves the roll width or overlaps another is refused (`planche_invalide`, with its number). Copies with no placement go in rows above.
- Variable data: one copy per CSV row (2000 at most; separator `;` or `,` taken from the header; double quotes). Each field writes the value of its column in black Helvetica, at x, y mm from the bottom-left corner of the page.
- Variable data can be combined with `gabarit` or `manuel`.
- Templates are included in the configuration backup.

## See also

- [Refine a sheet: shapes, repeat and overall outline](rip-affiner-une-planche.md)
- [Create a sheet to group prints](rip-creer-une-planche.md)
- [Back up the server configuration](../rip-administration/rip-sauvegarder-la-configuration.md)
