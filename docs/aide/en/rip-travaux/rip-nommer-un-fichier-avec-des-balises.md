---
titre: Name a file with tags
description: Have the file name carry the preset, machine, copies, scale and roll width through a tag pattern, for a click-free drop.
sous_rubrique: rip-travaux
position: 64
mots_cles: tag pattern, file name, preset, machine, copies, scale, roll width, separator
slug: rip-name-a-file-with-tags
traduction_de: rip-nommer-un-fichier-avec-des-balises
---

A tag pattern reads the name of the dropped file to pull out the preset and the rest. This article is for the administrator and the workstations that prepare files.

## Before you start

You need an administrator account to define the pattern. The pattern is set in the **Automatic folder rules** (*Règles du dossier automatique*) block (see [Set up the automatic folder](rip-regler-le-dossier-automatique.md)).

## Steps

1. In **System** (*Système*), **Automatic folder rules** block, enter the **Name tag pattern** (*Motif à balises du nom*), for example `{nom}__{machine}__{reglage}__{copies}`.
2. Click **Save** (*Enregistrer*).
3. Name your files according to the pattern: `Affiche__epson__bache__30.pdf` gives the Banner preset, the Epson and 30 copies.
4. Drop them in `entrees-auto/`.

   | Tag | Effect |
   |---|---|
   | `{nom}` | Free text, no effect. |
   | `{reglage}` | Saved preset, ignoring capitals, accents, spaces, `-` and `_` (`bache` = "Bâche"). |
   | `{machine}` | Machine from the catalogue, by identifier or name: the printer for automatic queueing, or the job's cutter. |
   | `{support}` | The Epson's media, by name. |
   | `{copies}` | Quantity of the job ticket (1 to 100,000). |
   | `{repetition}` | Number of prints of each page queued straight away (1 to 100). |
   | `{laize}` | Roll width of the preset's tiling, in mm (`1600` or `1600mm`). |
   | `{echelle}` | Scale in % (`200`, `12,5%`). |
   | `{largeur}` / `{hauteur}` | Wanted size in mm, for a PDF: the scale follows from it (the smaller of the two if both are given). |

## Good to know

- Each tag takes the text up to the separator that follows it; the last one goes up to the extension.
- The double separator `__` is recommended: a single `_` is often found in names.
- Two tags must be separated, and each can only be used once.
- A name that does not follow the pattern, or a value that cannot be read, is not refused: the job goes through the rules and carries an alert ("Nom hors du motif à balises", "Balise du nom non reconnue"; name outside the tag pattern, tag in the name not recognised), also written in the server log.
- Without a recognised `{reglage}`, the rules pick the preset; if no rule takes the file, the refusal reason gives these alerts.

## See also

- [Set up the automatic folder](rip-regler-le-dossier-automatique.md)
- [Fill in the job ticket](rip-renseigner-la-fiche-travail.md)
- [Drop a file in a hot folder](rip-deposer-un-fichier-dans-un-dossier-surveille.md)
