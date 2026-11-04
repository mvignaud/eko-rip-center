---
titre: Choose the Kona registration
description: Choose between P1-P4 crosses, a plain Mutoh frame, a frame with barcode or a multi-segment frame so the Kona reads the printed job and cuts.
sous_rubrique: rip-machines
position: 46
mots_cles: kona registration, mutoh frame, barcode, cutserver, p1 p4 crosses, multi-segment, epos 6213, arrow
slug: rip-choose-kona-registration
traduction_de: rip-choisir-le-reperage-kona
---

Registration adds around the image the marks the Kona reads in order to cut in the right place. This article helps you choose the right type. It is for operators.

## Before you start

The job is in **Print** mode (*Imprimer*) and the cutter is the Kona. The Kona is set to a resolution of **0.010 mm** (see [Connect the Mutoh Kona 1650](rip-brancher-mutoh-kona-1650.md)).

## Steps

1. Click **Import** (*Importer*) and choose the **Kona registration** (*Repérage Kona*):
   - **P1–P4 crosses** (*Croix P1–P4*): four crosses at the corners, 10 mm from the image. On the Kona: Actions > Contour Cut > Manual, then point to P1, P2, P3, P4.
   - **Mutoh frame (type 1)** (*Cadre Mutoh (type 1)*): 5 mm black frame, 10 mm from the image, inside at least 250 × 250 mm, 25 mm white margins on the sides, 30 mm at the front, 80 mm at the back (the minimum dimensions in the Kona manual are respected).
   - **Mutoh frame + barcode (type 2, CutServer SF)** (*Cadre Mutoh + code-barres (type 2, CutServer SF)*): the same frame and two POSTNET codes of 250 × 20 mm (bottom left, top right, 50 mm from the side and 9 mm from the frame) with the number in plain text. Inside at least 310 mm wide. This is the type proven in the workshop (printing, reading of the frame and codes, cutting).
   - **Multi-segment frame + barcode (type 3, CutServer MF)** (*Cadre multi-segments + code-barres (type 3, CutServer MF)*): 125 mm corners, a 30 mm mark at most every 500 mm on each side, same codes, for long lengths cut in segments.
2. For type 2 or 3, download the cut file: it has the name Mutoh CutServer expects, `Name_SF_<number>.plt` (type 2) or `Name_MF_<number>.plt` (type 3). The number (11 digits) comes from the job and the page.
3. Drop the file in the CutServer folder: the Kona reads the code and CutServer sends the file with the same number.
4. Feed in the Kona the edge shown by the ▲ arrow on the frame (inner bottom-right corner): the edge that comes out of the Epson first, fed in from the back.

## Good to know

- ⚠️ The Kona cannot read a frame without a barcode on its own (manual, section 6.2): a type 1 sent directly stops on "EPOS 6213". For automatic reading, use type 2.
- **Automatic multi-frame**: with type 2, a page of **1.5 m or more** in the feed direction automatically takes the multi-segment frame (type 3), and its file gets the `MF` suffix. The Kona manual (section 6.3) advises against the plain frame beyond 2 m and the multi-frame under 1.5 m.
- Types 2 and 3 follow the tables in the Kona application guide; the position of the top code and the spacing of the type 3 marks are read from its diagrams, which have no dimensions. Type 3 is still to be proven on the machine.
- **Registration alerts**: an image under 25 cm on a side is flagged (recommended size for Kona registration).
- With a Kona registration, the cut lines and crosses are not printed (see [Add eyelets and printed marks](../rip-travaux/rip-poser-des-oeillets-et-des-marques-imprimees.md)).
- The printed page is enlarged accordingly, and the cut file follows the same offset.
- On dark, transparent or reflective media, see [Print a background under the marks](../rip-couleur/rip-imprimer-un-fond-sous-les-reperes.md).

## See also

- [Set up the Kona cut](rip-regler-la-coupe-du-kona.md)
- [Connect the Mutoh Kona 1650](rip-brancher-mutoh-kona-1650.md)
- [Print a background under the marks](../rip-couleur/rip-imprimer-un-fond-sous-les-reperes.md)
