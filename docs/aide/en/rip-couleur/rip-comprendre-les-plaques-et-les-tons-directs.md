---
titre: Understand ink plates and spot colour names
description: Learn how the RIP splits a file into plates, which names give white or silver, and what happens to cut spot colours.
sous_rubrique: rip-couleur
position: 40
mots_cles: plates, white, silver, metallic, spot colours, none, recognised names
slug: rip-understand-ink-plates-and-spot-colour-names
traduction_de: rip-comprendre-les-plaques-et-les-tons-directs
---

The RIP renders one plate per ink. This article lists the names the RIP recognises, so you can prepare a file without surprises.

## Before you start

You are preparing a file in your layout software or checking a rendered job.

## Steps

1. Name the file's spot colours according to this table.

   | Plate | Spot colour name |
   |---|---|
   | Cyan, Magenta, Yellow, Black | The four basic inks of the file. |
   | **White** | `White`, `Blanc` or `WH` (Epson recommends `White`). |
   | **Metallic silver** | `Silver`, `Metallic`, `Argent` or `MS` (Epson recommends `Metallic`). |
   | Other spot colour | One plate per colour of the PDF; to be converted to CMYK or declared in the spot colour table. |

2. Send the file, then click the job and open **Layers** (*Couches*) in the inspector: one thumbnail per plate.
3. On a plate's preview, black is the ink and white is the absence of ink.
4. To lay the white or silver under or over the colour (window graphics), use **White or silver** (*Blanc ou argent*) in the job's White section.

## Good to know

- Cut spot colours (`CutContour`…) are never printed and do not erase the ink under them, even without overprint in the file: at render time, they become the `None` colorant of the PDF standard. So there is no white edge along the cut.
- Spot colours that have their own ink (`White`, `Silver`…) are never converted to CMYK.
- Queueing is refused, with the reason, for any other spot colour (convert it to CMYK, rename it, or add it to the spot colour table), or for white and silver on the same page.
- The `MT-*` metallic colours of the Epson swatch book are not converted yet.
- To generate the white without drawing it, see [Lay down white automatically](rip-poser-du-blanc-automatique.md).

## See also

- [Declare the workshop's spot colours](rip-declarer-les-tons-directs.md)
- [Lay down white automatically](rip-poser-du-blanc-automatique.md)
- [Understand the Epson direct driver](../rip-machines/rip-comprendre-le-pilote-direct-de-l-epson.md)
