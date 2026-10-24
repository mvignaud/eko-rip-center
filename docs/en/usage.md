# Usage

The screen has three views, at the top: **Jobs**, **Queue** and **System**. The button
on the right switches language (French or English). If the server has accounts, the
screen asks you to sign in first (see *Security* in the server installation guide).

## The screen

Work happens in the **E-KO RIP Center client**. The screen reads like a RIP queue: the
**toolbar** on top (**Import**, then actions on the selected job: **Print**, **Cut**,
**Edit**, **Proof**, **Retry**, **Delete**; search, theme ◐, language), on the left the
**views**, the catalogue **machines** with their queued outputs and the **saved presets**,
in the middle the **job table**, on the right the **inspector** of the selected job.

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

## Quick edit

**Quick edit** block of the send panel, or **Edit** button on a finished job (rendering
is redone from the original file, no need to send it again):
- **Scale** (%), **Rotation** (0, 90, 180, 270°), **Mirror** (window film, lightbox
  seen from behind), **White margin** around the image;
- **Grommets**: printed marks (ring and centre cross) along the ticked edges, at the
  given **distance from edge**, corners included, evenly spread with a **maximum
  spacing**;
- **Rectangular cut contour** around the image, offset by a few mm (negative = inside).

Cutting follows the edit. A job with an output in the queue cannot be edited: cancel or
finish the output first.

## Automatic queueing

**Automatic queueing** block: once rendered, each printable page goes to the Epson
queue and each page to cut to the Kona queue. **Add on hold** holds them: import ten
files and release the ones you want from the queue. A cut that follows a print is
always added on hold (the media must first move to the Kona). The setting is saved with
the others: a hot folder can thus print on its own.

Several files can be chosen at once; each is sent with the same settings.

## Proof (BAT)

**Proof (PDF)** link on a finished job: an A4 landscape PDF to send to the customer, one
page per job page: the image soft-proofed with the media profile, the cut line in magenta
(not printed), the size in mm, the settings (mode, resolution, profile, white, edit,
grommets), the preflight warnings and an “Approved for printing” box (date, name and
signature). It comes out in the screen's language.

## Viewer

Clicking a thumbnail (composite, plate, cut) opens it large, with its size in mm.

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
  They are laid out as the driver will send them: one file per printer channel
  (`K.tif`, `C.tif`, `M.tif`, `Y.tif`, then `WH.tif` or `MS.tif`) and `envoi.json`
  (channels in order, source plate, layers). Cut spot colours (`CutContour`…) never go
  out. White is recognised as `White`, `Blanc`, `WH`, silver as `Metallic`, `Silver`,
  `Argent`, `MS` (Epson recommends `White` and `Metallic`). **White or silver** (White
  section of the job) puts it under the colour or over it (window film). Queuing is
  refused, with the reason, for any other spot colour (convert it to CMYK or rename it),
  or for white and silver on the same page.

Each print still to run shows its **estimated time**, and the column header the total
left; so does the job card. Estimate: printed length × machine roll width ÷ speed. Epson
default speed (Epson data sheet): 94 m²/h up to 360 dpi, 17 m²/h at 720 dpi (720 × 1440,
6 passes), 6.6 m²/h above; **Speed (m²/h)** in the machine editor overrides it (measure
it in the workshop, and for the Roland). When an output finishes or fails, the screen
says so, and the system too if the client may show notifications.

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
  (minimum sizes from the Kona manual). ⚠️ The Kona does not read a frame without a
  barcode by itself (manual, § 6.2: manual, Single Frame, Multi Frame): sent directly,
  it stops on "EPOS 6213". For automatic reading, use type 2.
- **Mutoh frame + barcode (type 2, CutServer SF)**: **proven in the workshop on 24/10**
  (print, frame and code reading, cut). The same frame and two 250 × 20 mm POSTNET codes (bottom left, top right, 50 mm from the
  side and 9 mm from the frame) with the number in clear. The 11-digit number comes from
  the job and page. The downloaded cut file has the name Mutoh CutServer expects,
  `Name_SF_<number>.plt`: drop it in its folder, the Kona reads the code and CutServer
  sends the file with the same number. Inside at least 310 mm wide.
- **Multi-segment frame + barcode (type 3, CutServer MF)**: 125 mm corners, a 30 mm
  marker at most every 500 mm on each side, same codes, file `Name_MF_<number>.plt`, for
  long jobs cut segment by segment.

Types 2 and 3 follow the tables of the Kona application guide (ch. 4.4.2); the place of
the top code and the type 3 marker spacing are read from its drawings, without
dimensions (type 3 still to be proven).

Every frame carries an **▲ arrow** in its inner bottom-right corner: it points to the
edge that leaves the Epson first, the one fed first into the Kona (from the rear).

**Sending to the Kona.** The RIP sends the cut as Mutoh CutServer does: with a frame it
declares two opposite frame corners (line centres), the Kona scans it, reads the code,
then cuts. **Cut settings** (speed in cm/s, force in g, acceleration in G) are declared
per machine in System > Machines, like CutServer's "cut layers"; the first is the
default. As in CutServer, each can also keep the **cutter defaults** (nothing sent),
force a **tool** (1 to 4), and follow a cut **pattern**: up to 10 "down/up" pairs in mm
(`8/2 8/2` = cut 8 mm, lift 2 mm), with **spread** (a whole number of patterns per
straight line, the cut starting and ending at sharp corners). **.plt file** with `?machine=kona` gives the file as the Kona receives it, to
send by hand (serial port 9600, 8N1, RTS/CTS). If the cut comes out turned 180°, tick
**cut turned** on the machine.

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
by default) and the **spacing**, then **Build the sheet**. Before it is built, the panel draws the **imposition**: the roll
at its width, each item in place (hover: name and size), the roll length and the share
of area used; it updates on every change. With **Rotate 90° when the sheet gets shorter**, four
orientations are tried (as is, all rotated, all lying, all standing) and the shortest on
the roll is kept; an item wider than the roll only fits rotated. The resolution and
registration of the New job panel apply. Items are placed tallest first, left to right;
the sheet is a new job, with all contours cut.

With **Nest on the shape of the cut contours**, each item takes the room of its cut
contour plus the spacing instead of its rectangle: circles staggered, triangles head to
tail (with rotation, angles 0, 90, 180 and 270° are tried). An item without a closed
contour keeps its rectangle. The imposition then draws the contours. Nesting runs on a
grid whose step is 1/800 of the roll width (1 mm at least): a cut never touches its
neighbour, but the actual gap may exceed the requested spacing by one step. The share of
area used counts the area of the cut contours, in both modes.

**Step & repeat pattern** repeats one page in a grid: **columns** × **rows**, at the
chosen spacing. With **Half drop**, every other column moves down half a height
(wallpaper repeat). The ticked number of copies is ignored; the grid must fit the roll
width (100 columns, 500 rows and 2000 copies at most). The cut follows every copy.

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

## Machines

**System > Machines**: the workshop's machine catalogue. For each: an id (lowercase,
digits, `-`), a name, the model (Epson SC-S80600, Mutoh Kona, Graphtec FC7000, Roland
VersaCAMM VP), the roll width, the connection (serial or USB-serial, TCP network, device
such as `/dev/usb/lp0`, or folder), the SNMP address to read the loaded media, and the cut
step (0.01 mm for the Kona, 0.025 mm by default elsewhere) and the cut **offset X / Y**
(−20 to 20 mm): the gap measured on a test cut between cut and print, added to every cut
sent to that machine. **Save** applies at once. A
machine with an output still to run cannot be removed.

- **Graphtec FC7000-75**: set the machine to HP-GL (Menu > Interface > Command), 0.025 mm
  step; ARMS registration is set on the machine's panel.
- **Roland VersaCAMM VP-300 / VP-540** (762 / 1371 mm): cutting goes over the network; to
  be tried at the workshop. Roland printing will follow a network capture.

## Media loaded in the Epson

**System > Media loaded in the Epson**: the width of the loaded roll, which the printer
measures when loading, is read over the network (SNMP, standard Printer-MIB) and becomes
the roll width of the Sheet and Tiling panels (**Use as roll width**, **Read again**).
Set the Epson's IP address in `EKO_RIP_EPSON_SNMP` (see the server installation guide).
To be validated on the machine: if the SC-S80600 does not publish its width in the
Printer-MIB, the screen says so, and reading will use another path after a capture.

## Ink and cost

Each printed page shows its area, estimated ink and cost:
`0.42 m² · ink 3.1 ml · ≈ €1.92 (ink €0.44, media €1.48)`. Under each plate thumbnail,
its ink rate (share of the page covered at 100 %).

The calculation: for each plate, *rate × area × ml/m² at 100 %* gives the ml, then
*ml ÷ 1000 × price per litre* the cost; media costs *area × price per m²*. Cut spot
colours are not counted.

Rates are set in **System > Ink and media costs**: one row per ink (ml/m² at 100 % and
€ per litre), an **All others** row for empty boxes, and the media price per m². The
shipped values (10 ml/m², Epson 700 ml cartridge at €123.90 incl. VAT i.e. €177/l, HEXIS V3000WG media at €1.97/m² on 1600 mm × 45 m rolls) are starting values: replace them with the workshop's
own (weigh cartridges before and after a known print, invoices). A rate change also
applies to jobs already done.

**Cartridges**: enter a cartridge's size (ml) and price; the price per litre follows.
**Media (rolls)**: a catalogue, each medium with its price per m², or its roll width,
length and price paid (the price per m² is computed). The medium is chosen on sending
(**Media (cost price)**, kept in presets); with a known width, the media counted is the
whole roll width over the printed length. The job card shows its **cost price** (ink +
media).

**Excl. or incl. VAT**: state whether the prices entered include VAT, the **VAT** rate
(20 % by default) and the basis of the **cost price shown** (excl. VAT by default); the
card, each page's estimate and the CSV (`base` column) follow.

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
