---
titre: Split pages and reject doubtful files on arrival
description: Create one job per PDF page, refuse files with a missing font, and understand the order of simultaneous renders.
sous_rubrique: rip-travaux
position: 65
mots_cles: one page per job, refuse if a font is missing, simultaneous renders, hot folder, refuses
slug: rip-split-pages-and-reject-doubtful-files
traduction_de: rip-trier-les-fichiers-a-l-arrivee
---

Two boxes in the send panel, kept in the preset, are mainly used for hot folders. This article is for the administrator.

## Before you start

You have a saved preset (see [Drop a file in a hot folder](rip-deposer-un-fichier-dans-un-dossier-surveille.md)).

## Steps

1. Click **Import** (*Importer*) and set the job.
2. For one job per page, tick **One page per job** (*Une page par travail*): a multi-page PDF becomes one job per page, named "fichier.pdf (p. 1/5)", "fichier.pdf (p. 2/5)"... with the same settings.
3. To refuse doubtful files, tick **Refuse if a font is missing** (*Refuser si une police manque*): the file is refused as soon as a font is not embedded, with the list of fonts.
4. Click **Save…** (*Enregistrer…*) to keep these boxes in the preset.
5. For a hot folder, find the refused files in `refuses/`, with their `.refus.txt` file.

## Good to know

- **One page per job** has no effect on a nested sheet.
- The PDF's 14 base fonts (Helvetica, Times, Courier, Symbol, ZapfDingbats) do not count: the render replaces them with fonts of the same dimensions.
- Without the **Refuse if a font is missing** box, the file goes through with an alert (see [Read a job's alerts](rip-lire-les-alertes-d-un-travail.md)).
- **Several renders at once**: the server renders several jobs at the same time (one per core, 4 at most, adjustable with `EKO_RIP_RENDUS`), shown in **System** > **Simultaneous renders** (*Rendus simultanés*).
- Arrivals from hot folders and from the ERP are queued in their drop order, even when a small file finishes rendering before a large one that arrived earlier.
- A file sent by hand from the screen goes ahead of automatic arrivals still waiting for their render, and goes as soon as it is ready.

## See also

- [Drop a file in a hot folder](rip-deposer-un-fichier-dans-un-dossier-surveille.md)
- [Set up the automatic folder](rip-regler-le-dossier-automatique.md)
- [Read a job's alerts](rip-lire-les-alertes-d-un-travail.md)
