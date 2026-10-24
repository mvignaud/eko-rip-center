# Usage

1. Open the screen (client or browser).
2. Pick a PDF and a resolution (360, 720 or 1440 dpi), then **Send to RIP**.
3. The job shows up in the list: *Queued*, *Rendering*, then *Done* with a preview
   of each plate.

A multi-page PDF shows one row per page, with its previews, its cut file and its
send-to-cutter button.

## Plates

One plate per ink: Cyan, Magenta, Yellow, Black, plus one plate per spot colour in
the PDF. For the Epson SC-S80600:

- **White**: name the spot colour `White` in Illustrator;
- **Metallic silver**: name the spot colour `Silver`.

On a plate preview, black is ink and white is no ink.

## Cutting

Paths painted in a cut spot colour are detected: `CutContour`, `Thrucut`, `Kisscut`,
`Insidecut`, `Perforation`, `Creasing`, `Score`, `Vcut` (case, spaces and hyphens do
not matter). The job card shows how many and offers **Download the .plt file**
(HP-GL, 1/40 mm, origin at the page's bottom-left corner). A cut colour at 0% is not
cut.

## Kona registration

In **Print** mode, **Kona registration** adds around the image:
- **P1–P4 crosses**: four crosses at the corners, 10 mm from the image; on the Kona,
  Actions > Contour Cut > Manual, then point P1, P2, P3, P4.
- **Mutoh frame (type 1)**: 5 mm black frame, 10 mm from the image, inside at least
  250 × 250 mm, 25 mm white on the sides, 30 mm at the front, 80 mm at the rear (the
  Kona manual minimums are met). The Kona measures it by itself.

The printed page grows accordingly and the cut file follows the same offset. Set the
Kona resolution to **0.010 mm**.

## Not there yet

Plates are not sent to the printer yet. The cut file is not sent to the cutter
automatically yet: send it by hand. The `MT-*` metallic colours
of the Epson swatch book are not converted yet.
