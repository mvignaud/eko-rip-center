---
titre: Set up a printer's nesting
description: Choose the strategy, loading, spacing, justification, purge strip and margins of a printer's automatic sheets.
sous_rubrique: rip-machines
position: 43
mots_cles: nesting, strategy, one by one, grouped, media saving, loading, justification, purge, nesttrimbox
slug: rip-set-up-printer-nesting
traduction_de: rip-regler-l-imbrication-d-une-imprimante
---

Instead of printing one by one, the waiting prints for the same printer can go out together on one sheet. This article describes a printer's settings, for the administrator.

## Before you start

You need an administrator account. Hovering over each line of the page shows its help.

## Steps

1. Open **Administration…**, the **System** view (*Système*), the **Machines** section, the printer's page.
2. Choose the **Nesting** (*Imbrication*) strategy:
   - **One by one** (*Un par un*) (default): each print goes out alone, nothing is nested;
   - **Grouped in order** (*Groupé dans l'ordre*): side by side, by priority (nearest deadline first), with no rotation or reordering;
   - **Media saving** (*Économie de support*): the whole sheet is recalculated each time a job arrives, tallest first, with the quarter turn when it shortens the sheet.
3. Choose the **Loading** (*Chargement*): **Free (roll width)** (*Libre (laize du rouleau)*) (the sheet takes the width of the loaded roll, otherwise that of the machine) or **Strict (one sheet per format)** (*Strict (une planche par format)*) (jobs of different formats do not share a sheet).
4. Set the **Spacing between copies (mm)** (*Espace entre copies (mm)*) (5 by default): white space between two items side by side, and between rows; **Spacing between rows (mm)** (*Espace entre rangées (mm)*) replaces it between rows (empty: the same).
5. Choose the **Justification**: **Left** (*Gauche*), **Centre** (*Centre*) or **Right** (*Droite*). Centred or right, the sheet takes the full roll width. If needed, set the **Origin offset X (mm)** (*Décalage d'origine X (mm)*) (0 to 1000): white space to the left of the sheet, before everything else.
6. Choose the **Purge strip** (*Bande de purge*): **None** (*Aucune*), **On the left** (*À gauche*), **On the right** (*À droite*) or **On both sides** (*Des deux côtés*), and the **Purge width (mm)** (*Largeur de purge (mm)*) (1 to 100, 10 by default).
7. Set **Top margin (mm)** and **Bottom margin (mm)** (0 to 200), and tick **Outline of the whole sheet (NestTrimBox layer)** if the sheet goes to a cutting table.
8. Tick **Flip every other panel** (*Retourner un lé sur deux*) for a colour match between neighbouring panels.
9. Set the start triggers (see [Set a sheet's start triggers](rip-regler-les-declencheurs-de-depart-d-une-planche.md)), then click **Save** (*Enregistrer*).

## Good to know

- A job's format: the roll width of its media in the catalogue, otherwise the one its tiling is made for; with no known format, the roll.
- Purge strip: it runs along the whole sheet, at the chosen edge (on the left, after the origin offset), in four equal cyan, magenta, yellow and black strips, to clear each head; the spacing between copies separates it from the items.
- **Flip every other panel**: the even panels of a tiling (2, 4…) are laid upside down (half turn, same place).
- The NestTrimBox outline surrounds the placed items (without the offset or the purge), in the `NestTrimBox` layer, cut last, never printed.
- What gets nested together and the sheet being prepared: see [Follow a sheet being prepared in the queue](../rip-travaux/rip-suivre-une-planche-dans-la-file.md).

## See also

- [Set a sheet's start triggers](rip-regler-les-declencheurs-de-depart-d-une-planche.md)
- [Follow a sheet being prepared in the queue](../rip-travaux/rip-suivre-une-planche-dans-la-file.md)
- [Prepare a sheet for a cutting table](rip-preparer-une-planche-pour-une-table-de-decoupe.md)
