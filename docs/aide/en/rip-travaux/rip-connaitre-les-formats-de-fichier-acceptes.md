---
titre: Check the accepted file formats
description: Check the formats, maximum size and handling of images, spot colour channels and the alpha channel before you send a file.
sous_rubrique: rip-travaux
position: 53
mots_cles: formats, pdf, ai, svg, tiff, png, jpeg, alpha channel, spot colours, 8 GB, ppi
slug: rip-check-accepted-file-formats
traduction_de: rip-connaitre-les-formats-de-fichier-acceptes
---

This article summarises what the RIP accepts, what it refuses and how it reads images. It is for anyone who prepares files.

## Before you start

Nothing special: you just need to know your file's format.

## Steps

1. Compare your file with this table.

   | Format | Notes |
   |---|---|
   | **PDF** | Reference format: all features (cutting, nesting, colour filter, auto contour). |
   | **AI** | Save with "Create PDF Compatible File" (*Créer un fichier compatible PDF*). |
   | **SVG** | For cutting only. |
   | **TIFF, PNG, JPEG** | Image placed at its real size from the resolution saved in the file; with no resolution, 72 ppi and the job flags it. CMYK, RGB or greyscale, 8 or 16 bit, with its ICC profile; transparency lays down no ink. |

2. Check the size: 8 GB at most per file (the server's `EKO_RIP_MAX_MO` setting).
3. For a TIFF with spot colour channels, name them in Photoshop: each becomes its own plate, laid over the colours without erasing them. An unnamed channel becomes `Spot1`, `Spot2`...
4. To get a white plate from a TIFF, name the channel `White`, `Blanc`, `WH` or `Spot1` (capitals, spaces and `_` are ignored).
5. After sending, look at the plates picked up, flagged by the job, before the first print.

## Good to know

- Not supported yet, refused with a message: palette TIFF, YCbCr, Lab or planar TIFF, interlaced PNG.
- Photoshop selection channels (masks) are not printed.
- The direction of a spot colour channel's values (clear channel = no ink) has not yet been checked on a workshop file: look at the plate before printing.
- An alpha channel (cut-out PNG or TIFF) can give a white or varnish plate: see [Lay white or varnish from an alpha channel](../rip-couleur/rip-poser-du-blanc-ou-du-vernis-depuis-un-canal-alpha.md).
- An image with no alpha channel (JPEG) has no such plate.
- If the file exceeds the limit, the message is "Fichier trop volumineux pour le serveur." (file too large for the server): see [Fix a failed render or a refused file](../rip-depannage/rip-un-rendu-echoue.md).

## See also

- [Send a job to the RIP](rip-envoyer-un-travail.md)
- [Understand ink plates and spot colour names](../rip-couleur/rip-comprendre-les-plaques-et-les-tons-directs.md)
- [Read a job's alerts](rip-lire-les-alertes-d-un-travail.md)
