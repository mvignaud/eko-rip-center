# Usage

1. Open the screen (client or browser).
2. Pick a PDF and a resolution (360, 720 or 1440 dpi), then **Send to RIP**.
3. The job shows up in the list: *Queued*, *Rendering*, then *Done* with a preview
   of each plate.

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

## Not there yet

Plates are not sent to the printer yet. The cut file is not sent to the cutter
automatically yet: send it by hand. The `MT-*` metallic colours
of the Epson swatch book are not converted yet.
