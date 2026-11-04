---
titre: Understand the cut preflight check
description: Read the cut alerts raised at render (RGB colour, path with fill, path on the edge, misaligned stroke...) and fix the file before cutting.
sous_rubrique: rip-travaux
position: 56
mots_cles: preflight, cut alerts, path, stroke, rgb, page edge, layer, open paths
slug: rip-understand-the-cut-preflight-check
traduction_de: rip-comprendre-le-controle-prealable-de-la-decoupe
---

At render time, the RIP checks the cut paths and flags what may cut badly. This article lists each alert and its fix.

## Before you start

The job has been rendered and shows the ⚠ badge (see [Read a job's alerts](rip-lire-les-alertes-d-un-travail.md)).

## Steps

1. Click the job and read the cut alerts.
2. Find them in the table and fix the original file.

   | Alert | Fix |
   |---|---|
   | Cut colour defined in RGB | Define it in CMYK, as a spot colour. |
   | Cut path with a fill | A cut path is a stroke only: remove the fill. |
   | Path touching the page edge | Move it inside the page. |
   | Path covered by the image | Put it on the top layer. |
   | Vectorised outline (stroke turned into a shape: it would be cut twice) | Keep the stroke, do not vectorise the outline. |
   | Stroke aligned inside or outside the path | Align it to the centre. |
   | Path going off the work area | What goes beyond is not cut: bring it back inside the area. |
   | Cutter chosen with no recognised cut spot colour | The file's spot colours are listed: rename the spot colour or add its prefix (see [Set cut prefixes](rip-regler-les-prefixes-de-decoupe.md)). |
   | Path on a non-printable layer | It is not cut, unless **Also cut paths on non-printable layers** (*Couper aussi les tracés des calques non imprimables*) is ticked (System > Cutting). |
   | Open paths not cut | Allow open paths, if open paths are not allowed (see [Set open paths, sorting and after-cut](rip-regler-les-traces-ouverts-le-tri-et-l-apres-coupe.md)). |

3. Send the corrected file again, or accept the alert: the job goes out anyway.

## Good to know

- These alerts add to those raised on sending (fonts, resolution).
- A page that carries cutting marks (Kona registration, frame...) but no cut path gets the alert: *Des repères de découpe sont imprimés, mais la page n'a aucun tracé de coupe : encre et support pour rien.* (Cutting marks are printed, but the page has no cut path: ink and media wasted.)
- A spot colour missing from the spot colour table has its own alert (see [Declare the workshop's spot colours](../rip-couleur/rip-declarer-les-tons-directs.md)).

## See also

- [Set cut prefixes](rip-regler-les-prefixes-de-decoupe.md)
- [Read a job's alerts](rip-lire-les-alertes-d-un-travail.md)
- [Auto contour an image](rip-detourer-automatiquement-une-image.md)
