# Usage

The screen has three views, at the top: **Jobs**, **Queue** and **System**. The button
on the right switches language (French or English). If the server has accounts, the
screen asks you to sign in first (see *Security* in the server installation guide).

## Sending a job

1. **Jobs** view, **New job** panel: drop a file onto the dashed area, or click it to
   choose one. You can also drop one or more files anywhere on the page: they are all
   sent with the panel's settings.
2. Pick the **Mode**:
   - **Print (and cut)**: rendered into ink plates, plus cutting when the file has cut
     paths;
   - **Cut only**: nothing is printed; only the paths go to the Kona.
3. Set the **Resolution** (360, 720 or 1440 dpi), the **Paths to cut** and the **Kona
   registration**. The **Colour** and **Tiling** blocks open with a click.
4. **Send to RIP**.

Accepted formats: PDF, AI (saved with “Create PDF Compatible File”) and SVG (cut only).

## Following jobs

Each job is a card: thumbnail, name, badges (state, mode or resolution, page count,
registration) and date. States: *Queued*, *In progress*, *Done*, *Failed*. Click a card
to open it:

- one row per page, with its number of cut paths;
- the **Print (Epson)** and **Cut (Kona)** buttons, which add the page to that
  machine's queue; the badge next to them shows the state of the last output;
- **.plt file**: downloads the cut file (HP-GL, 0.010 mm per step, origin at the
  bottom-left corner of the page);
- the previews: composite (soft-proofed with the media profile), cut, and one thumbnail
  per plate.

**Warnings** (orange badge): when a PDF to print is sent, the server flags fonts that
are not embedded (another one will be used at rendering: outline the text or embed the
font) and images under 100 ppi at their printed size. The job still goes through: the
operator decides.

The search bar filters by name; the **All**, **In progress**, **Done** and **Failed**
chips filter by state. **Retry** redoes a failed job; **Delete** removes a finished job
(refused while an output is waiting for it in the queue).

## Queue

**Queue** view: one column per machine. The number of outputs still to run shows on
the tab.

- **Mutoh Kona 1650**: the cut goes out through the cutter port (`/dev/kona`).
- **Epson SC-S80600**: its driver does not exist yet. Meanwhile, the plates to print
  are copied to `/var/lib/eko-rip/sorties/epson/<job>-p<page>-<output>/` on the server.
  Cut spot colours (`CutContour`…) are never copied there.

Each machine runs one output at a time, in its column's order. On an output still to
run: **To top**, **Up**, **Down**, **Hold** / **Resume**, **Cancel**. An output on hold
does not run; the next ones pass it. **History** keeps finished outputs: **Redo** runs
the same output again without re-rendering, **Remove** deletes it from the history.

An output cut short by a server shutdown does not restart by itself (a cut cannot
resume halfway): it becomes *Failed*, and **Redo** runs it again.

## Plates

One plate per ink: Cyan, Magenta, Yellow, Black, plus one plate per spot colour in
the PDF. For the Epson SC-S80600:

- **White**: name the spot colour `White` in Illustrator;
- **Metallic silver**: name the spot colour `Silver`.

On a plate preview, black is ink and white is no ink.

Cut spot colours (`CutContour`…) are never printed and never knock out the ink beneath
them, even without overprint in the file: at rendering they become the PDF standard's
`None` colorant. So there is no white hairline along the cut.

## Automatic white

**White** block (Print mode): the `White` plate is generated after rendering.
- **Underlay under the ink**: white wherever an ink exceeds the **minimum ink** (5 % by
  default) — for printing on clear or coloured media.
- **Flood over the whole page**: white over the whole page.
- **Choke** (0.2 mm by default, up to 5 mm): the white is pulled back by that much from
  the ink edge (and the page edge), so it never shows past the colour.

White already drawn in the file (`White` spot colour) is kept: the final white is the
union of both. The generated plate shows in the previews, counts in the cost and goes to
the Epson with the other plates. A saved preset keeps the white choice.

## Cutting

Paths painted in a cut spot colour are detected: `CutContour`, `Thrucut`, `Kisscut`,
`Insidecut`, `Perforation`, `Creasing`, `Score`, `Vcut` (case, spaces and dashes
ignored). A cut spot colour at 0 % is not cut. With **All paths**, every path in the
file is cut (cut-only files). Paths are clipped to the page: the blade never leaves the
media.

Cutting order: creasing (`Creasing`, `Score`), perforation and V-cut first, the cut last,
while the material still holds. A contour inside another (the hole of an “O”, a window)
is cut before it, so the piece does not move; at equal depth, the blade goes to the
nearest contour.

## Kona registration

In **Print** mode, **Kona registration** adds around the image:
- **P1–P4 crosses**: four corner crosses, 10 mm from the image; on the Kona, Actions >
  Contour Cut > Manual, point P1, P2, P3, P4.
- **Mutoh frame (type 1)**: 5 mm black frame, 10 mm from the image, inside at least
  250 × 250 mm, margins of 25 mm on the sides, 30 mm at the front, 80 mm at the rear
  (minimum sizes from the Kona manual). The Kona measures it by itself.

The printed page grows accordingly, and the cut file follows the same offset. The Kona
must be set to a resolution of **0.010 mm**.

## Presets and hot folders

**Save…** keeps the whole panel under a name (letters, digits, spaces, `-`, `_`, `+`).
Picking it in **Preset** fills the panel; **Delete** removes it.

Each preset has a **hot folder**: `/var/lib/eko-rip/entrees/<preset name>/` on the
server. A PDF, AI or SVG dropped there becomes a job with that preset once its copy is
complete (size stable for 5 s). A rejected file is moved to `refuses/`, with a
`.refus.txt` file giving the reason.

## Tiling

**Tiling** block: tick **Split into panels when wider than the roll**, set the roll
width and the **overlap** (area printed on both neighbouring panels, for gluing or
welding). All panels have the same width. Each panel becomes a page of the job, with
its own cut clipped to its edge. **Panel number and joining marks** adds a 10 mm band at
the foot of each panel: “Lé 2/3” and a vertical mark where the neighbouring panel stops
or starts, to line panels up when installing.

## Sheet (nesting)

On each printable page, tick **Add to sheet** and enter the number of copies. The
**Sheet** panel appears above the list: set the **roll width** (usable width, 1600 mm
by default) and the **spacing**, then **Build the sheet**. With **Rotate 90° when the sheet gets shorter**, four
orientations are tried (as is, all rotated, all lying, all standing) and the shortest on
the roll is kept; an item wider than the roll only fits rotated. The resolution and
registration of the New job panel apply. Items are placed tallest first, left to right;
the sheet is a new job, with all contours cut.

## Colour

**Colour** block (Print mode):
- **Add an ICC profile…** uploads a `.icc` or `.icm` profile to the server (media
  profiles, Epson Wide Gamut input profile…).
- **CMYK input / RGB input**: profile applied to untagged data (or to all data, with
  **Ignore the PDF's profiles**).
- **Media (output)**: profile of the printed media; the preview simulates it.
- **Intent**: perceptual, relative colorimetric, saturation, absolute.

Spot colours (`White`, `Silver`…) are never converted.

## System

**System** view: server version, job retention (30 days by default, set with
`EKO_RIP_CONSERVATION_JOURS`) and free space on the data disk, in red under 10 %.

## Ink and cost

Each printed page shows its area, estimated ink and cost:
`0.42 m² · ink 3.1 ml · ≈ €1.92 (ink €0.44, media €1.48)`. Under each plate thumbnail,
its ink rate (share of the page covered at 100 %).

The calculation: for each plate, *rate × area × ml/m² at 100 %* gives the ml, then
*ml ÷ 1000 × price per litre* the cost; media costs *area × price per m²*. Cut spot
colours are not counted.

Rates are set in **System > Ink and media costs**: one row per ink (ml/m² at 100 % and
€ per litre), an **All others** row for empty boxes, and the media price per m². The
shipped values (10 ml/m², €140/l) are starting values: replace them with the workshop's
own (weigh cartridges before and after a known print, invoices). A rate change also
applies to jobs already done.

**Export consumption (CSV)** downloads one row per printed page (UTC date, job, page,
area, ml per ink, costs), `;` separator, dot decimals, for a spreadsheet or E-KO ERP.

Known limit: the media counted is the page area, not the roll length used across the
whole roll width.

## Not there yet

- The Epson driver: plates go to a folder, not to the printer.
- Automatic frame reading by the Kona has not been tried on the machine yet.
- The Epson swatch book's `MT-*` metallic colours are not converted yet.

## Glossary

- **Plate**: 8-bit image of one ink (0 = full ink, 255 = no ink).
- **Spot colour**: named colour in the PDF (`White`, `Silver`, `CutContour`…), output on
  its own plate instead of being mixed into CMYK.
- **Roll width**: usable width of the roll.
- **Panel**: vertical strip of a tiled job, at most as wide as the roll.
- **Output**: one page of a job sent to a machine, in the queue.
- **Hot folder**: server folder where every dropped file becomes a job.
