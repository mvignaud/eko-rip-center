---
titre: Compensate for print bowing
description: Make the cut follow the curve of a print that comes out of the dryers bowed, using the media width, the start and the bow depth.
sous_rubrique: rip-travaux
position: 58
mots_cles: bowing, bow depth, dryers, curve, compensation, cut setup, media width
slug: rip-compensate-for-print-bowing
traduction_de: rip-compenser-le-cintrage
---

Once the dryers are hot, the print comes out curved (the middle lags behind the edges). The cut can follow this curve. This article is for operators.

## Before you start

The job is printed then cut. Make a measuring print: the compensation is set from it.

## Steps

1. On a print, measure the **bow depth** (*flèche*, in mm): how far the middle lags behind the edges, and the feed distance where the curve starts.
2. Open the job's **Cut setup** (*Mise en coupe*) panel.
3. Tick **Compensate for bowing** (*Compenser le cintrage*).
4. Enter the **Media width (mm)** (*Largeur du support (mm)*): the curve is centred on it.
5. Enter the **Start (mm of feed)** (*Début (mm d'avance)*), counted from the top of the page (nothing before it).
6. Enter the **Bow depth (mm)** (*Flèche (mm)*).
7. Click **Save the cut setup** (*Enregistrer la mise en coupe*), then make a test cut and adjust.

## Good to know

- The curve builds up over 50 mm.
- Values must be measured on a print from your workshop: there is no reliable default value.
- It does not replace the machine's **Offset X / Y** (*Décalage X / Y*).

## See also

- [Set up a job for cutting](rip-mettre-en-coupe-un-travail.md)
- [Fix a cut that is offset from the print](../rip-depannage/rip-decoupe-decalee.md)
- [Understand a machine's fields](../rip-machines/rip-comprendre-les-champs-d-une-machine.md)
