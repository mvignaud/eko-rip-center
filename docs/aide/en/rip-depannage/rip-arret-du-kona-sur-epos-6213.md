---
titre: Fix the Kona stopping on "EPOS 6213"
description: Understand why the Kona refuses to read a frame without a barcode, and switch to the type 2 frame.
sous_rubrique: rip-depannage
position: 40
mots_cles: epos 6213, kona, frame, barcode, type 1, registration, mutoh
slug: rip-fix-kona-stopping-on-epos-6213
traduction_de: rip-arret-du-kona-sur-epos-6213
---

The Kona stops and shows "EPOS 6213" instead of cutting. This article gives the cause and the fix, for operators.

## Before you start

The job was printed with a **Kona registration** (*Repérage Kona*) and sent directly to the Kona.

## Steps

1. Check the job's registration: click the job, then **Settings** (*Réglages*), and read **Kona registration** (*Repérage Kona*).
2. If it is **Mutoh frame (type 1)** (*Cadre Mutoh (type 1)*), that is the cause: the Kona cannot read a frame without a barcode on its own (manual, section 6.2).
3. Re-render with **Mutoh frame + barcode (type 2, CutServer SF)** (*Cadre Mutoh + code-barres (type 2, CutServer SF)*): click **Apply and re-render** (*Appliquer et refaire le rendu*).
4. Reprint, then drop the file `Name_SF_<number>.plt` into the CutServer folder: the Kona reads the code and CutServer sends the file with the same number.

## Good to know

- Type 2 is the one proven in the workshop.
- For a page of 1.5 m or more in the feed direction, type 2 automatically switches to the multi-segment frame (type 3).
- If the cut comes out shifted instead of stopping, see [Fix a cut that is offset from the print](rip-decoupe-decalee.md).

## See also

- [Choose the Kona registration](../rip-machines/rip-choisir-le-reperage-kona.md)
- [Connect the Mutoh Kona 1650](../rip-machines/rip-brancher-mutoh-kona-1650.md)
- [Fix a cut that is offset from the print](rip-decoupe-decalee.md)
