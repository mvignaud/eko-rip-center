---
titre: Lay down white or varnish from an alpha channel
description: Make a White or Varnish plate drawn from its alpha channel follow exactly the cut-out of a PNG or TIFF.
sous_rubrique: rip-couleur
position: 41
mots_cles: alpha channel, white, varnish, spot varnish, png, tiff, cut-out, plate
slug: rip-lay-down-white-or-varnish-from-an-alpha-channel
traduction_de: rip-poser-du-blanc-ou-du-vernis-depuis-un-canal-alpha
---

A cut-out PNG or TIFF carries an alpha channel that says where the image is opaque. The RIP can draw a white or spot varnish plate from it. This article is for operators.

## Before you start

Your file is a PNG or a TIFF with an alpha channel (a JPEG has none).

## Steps

1. Click **Import** (*Importer*) and choose the file.
2. Open the **White** (*Blanc*) block.
3. In the **Alpha channel** (*Canal alpha*) setting, choose a `White` plate (white) or a `Varnish` plate (spot varnish).
4. If needed, also set the generated white (underlay, solid): it is added to the alpha channel plate.
5. Click **Send to the RIP** (*Envoyer au RIP*), then open **Layers** (*Couches*) to check the plate.

## Good to know

- The plate follows the cut-out exactly: opaque = full ink, soft edge included, with no choke.
- An alpha that is opaque everywhere gives a plate over the whole image.
- If the TIFF already has a spot colour channel of the same name, that channel is the one that counts.
- An image with no alpha channel (JPEG) has no such plate.
- For white generated from the ink, see [Lay down white automatically](rip-poser-du-blanc-automatique.md).

## See also

- [Lay down white automatically](rip-poser-du-blanc-automatique.md)
- [Know the accepted file formats](../rip-travaux/rip-connaitre-les-formats-de-fichier-acceptes.md)
- [Understand ink plates and spot colour names](rip-comprendre-les-plaques-et-les-tons-directs.md)
