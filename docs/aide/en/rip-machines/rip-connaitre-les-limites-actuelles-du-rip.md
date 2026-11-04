---
titre: Know the RIP's current limits
description: Find what the RIP cannot do yet (white and silver on the direct driver, Roland printing, Kona type 3 frame, TIFF channels, metallic colours).
sous_rubrique: rip-machines
position: 49
mots_cles: limits, not yet, roland, white, silver, direct driver, type 3, tiff, metallic, roadmap
slug: rip-know-current-limits
traduction_de: rip-connaitre-les-limites-actuelles-du-rip
---

This article lists what is not yet supported, so you do not look for a feature that does not exist. The development order is public.

## Before you start

Nothing in particular. The roadmap is at https://e-ko.rip/feuille-de-route/.

## Steps

1. Compare your need with this list.

   | What does not exist yet | In the meantime |
   |---|---|
   | White and silver through the Epson direct driver | They come out to a folder (machine on a **Folder** connection, *Dossier*). |
   | Printing on the Roland | The RIP only sends it the cut. |
   | The Kona type 3 frame (multi-segment) has not yet been tried on the machine | Prefer type 2. |
   | Spot colour channels from a TIFF: the meaning of the values still has to be checked on a real Photoshop TIFF | Look at the plate before printing. The alpha channel, on the other hand, already gives a white or varnish plate. |
   | The `MT-*` metallic colours from the Epson colour chart | Not converted yet. |

2. For a machine that is not in the catalogue, see [Know the supported machines](rip-connaitre-les-machines-prises-en-charge.md).

## Good to know

- White and silver through an Epson on the network: queueing is refused with "Envoi à l'imprimante impossible" ("Cannot send to the printer"): see [Fix a failed output to the machine](../rip-depannage/rip-sortie-vers-la-machine-echouee.md).
- For white generated from the ink, see [Lay down white automatically](../rip-couleur/rip-poser-du-blanc-automatique.md).

## See also

- [Know the supported machines](rip-connaitre-les-machines-prises-en-charge.md)
- [Connect the Roland VersaCAMM VP-300 / VP-540](rip-brancher-roland-versacamm.md)
- [Understand the Epson direct driver](rip-comprendre-le-pilote-direct-de-l-epson.md)
