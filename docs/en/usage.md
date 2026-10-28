# Usage

The screen has three views, on the left: **Jobs**, **Queue** and **System**. The button
on the right of the toolbar switches language (French or English). If the server has
accounts, the screen asks you to sign in first (see *Security* in the server
installation guide).

## The screen

Work happens in the **E-KO RIP Center client**. The screen reads like a RIP queue: the
**toolbar** on top (**Import**, then actions on the selected job: **Print**, **Cut**,
**Edit**, **Settings**, **Proof**, **Retry**, **Delete**; search, theme ◐, language), on
the left the **views**, the catalogue **machines** with their queued outputs and the
**saved presets**, in the middle the **job table**, on the right the **inspector** of the
selected job.

## Sending a job

1. **Import**: drop a file onto the dashed area, or click it to choose one. You can also
   drop one or more files anywhere on the window: they are all sent with the import
   settings.
2. Pick the **Mode**:
   - **Print (and cut)**: rendered into ink plates, plus cutting when the file has cut
     paths;
   - **Cut only**: nothing is printed; only the paths go to the cutter.
3. Set the **Resolution** (360, 720 or 1440 dpi), the **Paths to cut**, the **Media and
   print mode** (see [Media and print modes](#media-and-print-modes)), the **Cutter**
   (see [Cutting](#cutting)) and the **Kona registration**. The **Colour**, **Quick
   edit**, **Automatic queueing**, **White**, **Background under marks**, **Cutting
   tolerance** and **Tiling** blocks open with a click.
4. **Send to RIP**.

Accepted formats: PDF, AI (saved with “Create PDF Compatible File”), SVG (cut only) and
**TIFF, PNG, JPEG** images. An image is placed at its real size, from the resolution saved
in the file (none: 72 ppi, and the job says so), in CMYK, RGB or greyscale, 8 or 16 bits,
with its ICC profile; transparency lays no ink. Maximum file size: 8 GB
(`EKO_RIP_MAX_MO`). Not supported yet: palette, YCbCr, Lab or planar TIFF, interlaced PNG
(refused with a message).

**A TIFF's spot colour channels**: each becomes its own plate, laid over the colours
without erasing them. Its name is the channel's name in Photoshop (no name: `Spot1`,
`Spot2`…). A channel named `White`, `Blanc`, `WH` or `Spot1` (case, spaces and `_`
ignored) gives the white plate. Photoshop selection channels (masks) are not printed. The
job lists the plates taken: look at them before the first print, the value direction
(light channel = no ink) has not yet been checked on a file from the workshop.

**Alpha channel → white or varnish plate**: in the White section of the send panel, the
“Alpha channel” setting can also turn a cut-out PNG or TIFF's alpha into a `White` plate or
a `Varnish` (spot varnish) plate that follows the cut-out exactly: opaque = full ink, soft
edge included, no choke. A fully opaque alpha gives a plate over the whole image. If the
TIFF already has a spot channel of the same name, that channel wins. An image without an
alpha channel (JPEG) gets no such plate. Generated white (underlay, flood) is added on top
if also chosen.

## Automatic rotation and settings after import

**Turn by itself when wider than the roll** (ticked by default): if the page,
registration included (frame, codes, margins), is wider than the printer's roll width
(System > Machines) but fits turned 90°, the RIP turns it and says so with a job alert.
No effect with a rotation chosen in quick edit or with tiling (the tiles handle it).

**Settings** (toolbar) reopens the import panel on the selected job: mode,
registration, cutter, tiles, white, media, colour… **Apply and render again**
starts from the original, without sending the file again.

## Quick edit

**Quick edit** block of the send panel, or **Edit** button on a finished job (rendering
is redone from the original file, no need to send it again):
- **Scale** (%), **Rotation** (0, 90, 180, 270°), **Mirror (window film)** (lightbox seen
  from behind), **White margin** around the image. **The mirror flips**: **the image and
  the cut** (default) or **the image only** — to be proven in the workshop: depending on
  how it is mounted, the cut must follow the image or not;
- **Grommets (printed marks)** along the ticked edges, at the given **distance from
  edge**, corners included. **Placement** **by spacing** (a **maximum spacing**) or **by
  quantity** (a number per edge). **Shape**: circle, filled disc, cross, circle and
  cross. With **Distance measured from the cut line (inwards)**, the inset starts from the
  cut contour, not the edge;
- **Rectangular cut contour**, offset by a few mm (negative = choke inwards), **around**
  the image, the bleed, or the trim box (bleed + trim);
- **Printed marks** (see below), **Bleed**, **Printed label** and **QR code in the
  margin**.

Cutting follows the edit. A job with an output in the queue cannot be edited: cancel or
finish the output first. The margin grows by itself to hold the marks, the label and the
code.

**Printed marks**:
- **Corner crop marks**, at an adjustable **distance from the bleed** and **length**; or
  **Corner registration crosses** (they replace the crop marks);
- **Contour printed on the cut line**;
- **Fold and sewing marks**: **Add a mark**, choose the side and the **distance from the
  edge**; **Sewing (continuous line)**; with **Printed** unticked the mark is not printed
  (20 marks at most);
- **Line** solid, dashed or alternate (dashes on white), with **Dash**, **Gap**, **Width**
  and **Colour** (black, registration CMYK 100 %, white).

With Kona registration, crop marks and crosses are not printed (they would disturb the
reading of the marks): a job alert says so.

**Bleed** on all 4 sides (**Sides linked** for one value everywhere), up to 100 mm.
**Fill**: a colour (C, M, Y, K in %), the first edge pixel, a mirrored image, or edge
duplication. Cut spot colours are not copied into the bleed. **Trim white**: white added
around, beyond the bleed. Marks sit outside the bleed.

**Printed label**: name, date, W × H, panel, copies, mode and order, at the top or bottom
of each panel, at the chosen **text size**. Copies and order reference print empty
(“—”) until the job sheet exists.

**QR code in the margin**: top or bottom, **code size**, **white around**, **error
correction** level (L, M, Q, H) and the **tags** to put in it (name, page, tile/total,
date, job ID, cut ID, machine, order ref.). Optional **Cut contour around the code**.

## Automatic queueing

**Automatic queueing** block: once rendered, each printable page goes to the Epson
queue and each page to cut to the Kona queue (or the cutter chosen at import). **Add on
hold** holds them: import ten files and release the ones you want from the queue. A cut
that follows a print is always added on hold (the media must first move to the Kona). The
setting is saved with the others: a hot folder can thus print on its own.

Several files can be chosen at once; each is sent with the same settings.

## Proof (BAT)

**Proof** button on a finished job: an A4 landscape PDF to send to the customer, one
page per job page: the image soft-proofed with the media profile, the cut line in magenta
(not printed), the size in mm, the settings (mode, resolution, profile, white, edit,
grommets), the preflight warnings and an “Approved for printing” box (date, name and
signature). It comes out in the screen's language. A colour filter is mentioned, but the
proof image does not show it.

**Proof on another machine**: with at least two printers, the page's **Proof on…** list
(next to **Print on…**) picks the proof machine. **Print on** then queues two outputs: the
**"name-BAT"** copy on the proof machine, and the requested print, **on hold**, showing
**Waiting for the proof** with the proof's state. Once the proof is approved, **Resume**
the original; rejected, **Cancel** it. Proofs are prints only, on a machine other than the
requested one (nor of the same pool). The choice stays until changed: back to **No
proof**.

## TIFF proof

The **TIFF proof** toolbar button downloads a TIFF of the page being viewed, made from the
rendered plates, that is from what goes to the machine. The page is at its exact size: the
proof also serves to check a panel's dimensions before printing it.

Settings, in the **TIFF proof** section of the send panel (kept in a saved preset):

- **Colour**: **Device (CMYK)**, the ink values as sent, with the media profile if there is
  one; or **Colorimetric (RGB)**, converted with the media profile (relative colorimetric
  intent), to see the result on screen.
- **Spot colours** (white, silver, varnish…): **kept as channels** (read back as spot
  colours by Photoshop and by the RIP), **flattened into the colour** (from a display
  colour: white as light grey, silver as grey), or **removed**. Cut tones are never
  included.
- **Maximum size** (1 to 3500 MB, uncompressed): above it, the resolution drops (one pixel
  for 2 × 2, 3 × 3… pixels), the physical size stays exact.
- **Make the TIFF proof of each page after rendering**: it is ready to download as soon as
  rendering ends. Otherwise, the button makes it on demand.
- **Proof only**: the proof is made after rendering and nothing is sent to print (no
  automatic queueing, no Print button; queueing a print is refused). The job can still be
  cut.

A proof that fails does not stop the job: it is listed in its alerts.

## Visual editor (viewer)

Clicking a thumbnail (composite, plate, cut) in the inspector opens it large.

**Viewing.** Zoom from 25 to 800 % with the wheel (the point under the cursor stays put),
with **−**, **+**, the % list, **Fit** and **100 %** (actual size on screen). **Hand**
tool (drag to move) and **Magnifier** (click: zoom in; Shift+click: zoom out; drag a
rectangle: zoom on it). Rulers in mm on two edges; at the bottom, the page size and the
cursor position in mm. Keyboard: `+` `−` `0` `1` and arrows.

**Roll width.** **Roll width** box: the width of the media loaded in the Epson (otherwise
the machine's roll width) is drawn; what overflows is in red, with the gap in mm.

**Pages.** A multi-page job shows a strip of thumbnails, one per page, the current one
marked. ← → or PgUp PgDn change page; the chosen view (composite, cut, plate) is kept.

**Selection.** A click on the page selects it (frame and 8 handles); a click outside the
page deselects it.

**Layers.** Right-hand panel: image, cut (in magenta), white and silver, switched on one
by one. White and silver are the plates with the names the Epson recognises, tinted on
the image. Kona marks are printed in the Black plate: no separate layer. The cut is only
overlaid when the cut is placed like the print (no rotation or mirror in [cut
placement](#cut-placement)).

### Editing the page

The editing tools only open on a rendered PDF without tiling. Each change is prepared on
screen; **Apply** sends it and renders again, **Discard changes** goes back to the
rendered state.

- **Crop**: handles (drag an edge, drag inside to move) or fields X (from the left), Y
  (from the bottom), Width, Height, in mm of the original page, before rotation and
  mirror. 9-point **Anchor**, **Keep proportions**, **No crop**. A frame that goes outside
  the page is refused (at least 1 × 1 mm).
- **Size**: **Printed size** as width and height (mm) or %. Corners: proportional;
  sides: free. With **Proportions linked** unticked, width and height are set apart, and
  the cut is distorted the same way. The top left corner stays put.
- **Rotate 90° left** (⟲) / **right** (⟳), **Mirror**; the **The mirror flips** choice
  (image and cut, or image only) shows with the mirror — to be proven in the workshop. A
  mirrored job carries the **Mirror** badge in the queue.
- While cropping or resizing with Kona registration, the marks are drawn dotted around the
  size to come.

**Undo / redo**: ↶ and ↷, or Ctrl+Z (Cmd+Z), Shift+Ctrl+Z and Ctrl+Y; 50 states at most.
Going back to a state already rendered renders again; a state only typed in comes back
into the panel.

### Colour filter

**Colour** button in the viewer: **Colour filter** block, specific to the job. It acts on
the CMYK plates, after the output profile and before automatic white; white, silver and
cut are left untouched.
- **Curves** C, M, Y, K: “input:output” points in % (e.g. `0:0 50:60 100:100`), 16 at
  most; empty = none.
- **Brightness** (+ = less ink) and **Saturation**, from −100 to 100 %.
- **Colour replacement**: a CMYK colour **From** to a CMYK colour **To**, with a **Range**
  (± %), 20 at most; it comes before the rest of the filter. **Add a replacement**, **On**
  box, **Remove**.
- **Before / after (left: before)**: split preview, on a filter already rendered; the
  slider moves the split. “After” estimates the effect on the soft proof.
- **Export to the preset**: the filter goes into a saved preset, or a **New preset (this
  job's settings)**; it is picked up at import. **No filter** removes it.

### Panels and tiles

On a page wider than the roll, the viewer shows the panels dotted, their overlaps tinted
and their numbers. **Panels** panel:
- drag a panel (or row) line for unequal panels; **Panel widths** and **Row heights** can
  also be typed in mm; **Equal panels** goes back to an even split;
- **Roll width**, **Maximum tile height** (beyond it the page is split into rows) and
  **Overlap** give the grid, summed up as columns × rows;
- **Weld**: no ink in the overlap under the neighbouring tile (at least 12.7 mm of
  overlap);
- **Cut line in the middle of each overlap**;
- **Tiles to print**: one box per tile; an unticked tile is neither rendered nor queued.
  Clicking its number sets its own **overlap** (negative = gap);
- **Print order** (e.g. `3 1 2`): these tiles first, then the others by number.

**Split into panels** applies. On a job already in panels, **Apply** keeps the changes
and **Whole page (no tiling)** goes back to a single page.

## Following jobs

**Production** view (top bar; **System** next to it), in three areas:

- **Machines** (top): one row per machine, with its state (ready, printing and the output
  running, unreachable), loaded media, the Epson's ink levels, the number of queued outputs,
  their spool weight (the plates to send) and the time left. A click shows only that
  machine; a second click shows them all again (same choice as **Show**). Under the table:
  the whole queue's spool and the data disk (% used, free space, gauge), in red when free
  space drops below the purge threshold (`EKO_RIP_SEUIL_LIBRE_MO`). These counters refresh
  with the queue.
- **To print**: the queued outputs, in queue order, then the jobs not sent yet ("Not
  sent"). Columns: thumbnail, name, state, machine, size, due date, time, date. Nests in
  preparation show above. The button column on the left acts on the chosen output: **To
  top**, **Up**, **Down**, **Hold**, **Resume**, **Cancel**, **Redo**, **Restore**,
  **Remove** (a greyed button does not apply to its state).
- **Printed** (collapsible, with their count): finished, failed or cancelled outputs,
  latest first, with the end time and the ink and media reading.

On the right, the **inspector** has four tabs: **Job** (the detail below), **Machine**
(media, inks, heaters, alerts of the machine shown or of the chosen output), **Settings**
(drop a file on a setting to import it with that setting; a click opens the import with
it) and **Log** (the screen's messages, latest first). Below 1100 px, the inspector moves
under the tables; below 1360 px, the bar's tools keep their symbol and their name goes to
a tooltip.

**Right click**: on a row, the output's queue actions then the toolbar's; on a machine,
show only it, see its state, re-read the Epson, configure it; on a table header, the
columns to show or hide (name, state and thumbnail stay; the choice is kept on this
computer). **Drag and drop**: in **To print**, a queued or held output takes the place of
another one of the same machine.

Click a row to open its job in the inspector:

- one part per page, with its number of cut paths;
- a **Print on…** button per printer (and per pool) and **Cut on…** per cutter, which add
  the page to that machine's queue; the badge next to them shows the state of the last
  output;
- **.plt file**: downloads the cut file (HP-GL, 0.010 mm per step, origin at the
  bottom-left corner of the page);
- the preview (composite, soft-proofed with the media profile); the cut and one thumbnail
  per plate are behind the **Layers** button;
- on a job to cut: the **Cut layers** and **Cut placement** panels, and on each printed
  page of a PDF, **Auto contour** (see [Cutting](#cutting)).

**Warnings** (⚠ badge): when a PDF to print is sent, the server flags fonts that are not
embedded (another one will be used at rendering: outline the text or embed the font) and
images under 100 ppi at their printed size. The cut preflight adds its own (see
[Cutting](#cutting)). The job still goes through: the operator decides.

**Job ticket** (inspector, folded under its title, with its summary): **Order no.**,
**Customer**, **Company**, **Quantity due**, **Due date** (tick box, then the date),
**Operator** and **Notes**; **Save the ticket**. 200 characters per field, 2000 for the
notes, quantity 100,000 at most. The quantity due is the number of **copies** placed on
the nesting sheet and is printed on the label with the order no.; the **due date** is the
nesting priority (nearest first, then jobs without one, then order of arrival). The table
has a **Due** column, and the **Sort** choice (**Arrival** or **Due date**) above it.

**Print this page** (tick box on each page of the inspector, ticked by default):
unticked, the page goes into neither the queue nor a nesting sheet (**Not printed**
badge); an output already queued waits with the reason **Page excluded**. Ticked again,
it comes back. An unticked page cannot be queued by hand either.

The search bar filters by name, order number, customer or company. **Retry** redoes a failed job; **Delete** removes a finished job
(refused while an output is waiting for it in the queue).

## Queue

Each machine's queue reads in the **Production** view (**To print** area, filtered by
machine). The number of outputs still to run shows on the **Production** button.

- **Mutoh Kona 1650**: the cut goes out through the cutter port (`/dev/kona`).
- **Epson SC-S80600**: with a **Network (TCP)** connection, port 9100, the RIP prints
  through its own driver (see [Direct Epson driver](#direct-epson-driver)). With a
  **Folder** connection, nothing is printed: the plates are copied to
  `/var/lib/eko-rip/sorties/epson/<job>-p<page>-<output>/` on the server, one file per
  printer channel (`K.tif`, `C.tif`, `M.tif`, `Y.tif`, then `WH.tif` or `MS.tif`) and
  `envoi.json` (channels in order, source plate, layers); the finished output shows
  **Left in the folder (not printed)**.

Cut spot colours (`CutContour`…) never go to the printer. White is recognised as
`White`, `Blanc`, `WH`, silver as `Metallic`, `Silver`, `Argent`, `MS` (Epson recommends
`White` and `Metallic`). **White or silver** (White section of the job) puts it under the
colour or over it (window film). Queuing is refused, with the reason, for any other spot
colour (convert it to CMYK or rename it), or for white and silver on the same page.

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
**Archive** keeps archived cuts (see [After cutting](#cut-on-arrival-and-after-cutting)):
**Restore** or **Remove**.

**Wait reason.** An output that cannot go says so with a red badge (hover for details),
and lets the next ones pass (an output held by an operator simply stays *On hold*):
- **Machine unreachable**: the machine is unplugged or missing from the catalogue; the
  output leaves as soon as it answers;
- **File missing**: rendering or cut path not found; re-run the job;
- **Too wide**: the page, marks included, is wider than the machine's or the loaded
  roll's width;
- **Other media**: the loaded media is not the job's; the output leaves by itself once
  the right one is loaded;
- **Print not finished**: a cut waits until its page is printed;
- **Cutter unavailable**: a print waits until the assigned cutter is usable (no wasted
  ink);
- **Nesting**: the print waits for its sheet (see [Nesting in the
  queue](#nesting-in-the-queue));
- **Page excluded**: the page is unticked (**Print this page**).

A print placed on a sheet becomes **Nested** ("on" the sheet's name): it no longer leaves
alone, the sheet prints it. It cannot be **Redone** until its sheet is printed (that would
print it twice); **Remove** is still possible.

An output cut short by a server shutdown does not restart by itself (a cut cannot
resume halfway): it becomes *Failed*, and **Redo** runs it again.

## Direct Epson driver

With an Epson SC-S80600 on a **Network (TCP)** connection, the printer's **IP address**
and **TCP port** 9100 (System > Machines), the RIP sends the print itself: the C, M, Y, K
plates of the rendering are split over the machine's 9 inks (light/dark pairs, ink
limit), screened with 4 dot sizes and sent band by band. First CMYK flat printed on
25/10, colours right.

- **Printer alerts**: with an **SNMP** address on the machine, the state is read before
  sending, then every 5 s until printing ends. A blocking alert (cover open…) stops the
  sending: the output becomes *Failed* with “The printer reports an alert: printing
  stopped”. Heating up counts as printing. Without SNMP, the output is done as soon as
  everything is sent.
- **Blank head**: the image starts 20 mm after the start of the page, to avoid the dark
  band seen at the head of the first print.
- **Ink ceiling**: each channel is capped at 85 %, against the thin bands seen in the
  yellow on the first print. Blank head and ceiling are to be confirmed on the second
  print.
- **Print origin X / Y (mm)** of the machine: places the image on the page sent, X from
  the reference edge of the roll (rounded to 16 dots), Y added to the blank head. 0 by
  default, to set on a paper test.
- **White and silver** do not go through the direct driver yet: queuing to an Epson on
  Network is refused (“Cannot send to the printer”); use a machine on **Folder**
  meanwhile.

Known limits: only the prints of 25/10 have been made; other resolutions and real media
are still to be proven in the workshop.

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
the Epson with the other plates (through a Folder connection for now, see the direct
driver). A saved preset keeps the white choice.

## Cutting

Paths painted in a cut spot colour are detected. A cut spot colour at 0 % is not cut.
With **All paths**, every path in the file is cut (cut-only files). Paths are clipped to
the page, and to the box of a PDF form: the blade never leaves the media.

### Cut prefixes

**System > Cutting**, **Cut prefixes (separated by ;)**: a spot colour is a cut one if
its name **starts** with a prefix. Default: `CutContour`, `Thrucut`, `Kisscut`,
`InsideCut`, `Perforation`, `Creasing`, `Score`, `VCut`. Case, spaces and dashes ignored:
`CutContour` matches `CutContour1` and `CutContourKissCut`, not `1CutContour`. A change
applies to the next renderings: restart a job already rendered.

### Cut layers

**Cut layers** panel of the inspector: **one layer per spot colour name**, with its number
of paths.
- **The layer order is the cutting order** (↑ / ↓): kiss cut before through cut. Starting
  order: creasing (`Creasing`, `Score`), perforation and V-cut first, the cut last, while
  the material still holds.
- The layer's box cuts it or not.
- **Cut setting**: by default **— same name, else default —**: the cutter's setting named
  like the layer applies by itself, else the machine's first one. Another one can be
  forced.
- Nothing is kept without **Save layers** (the cut files are then rebuilt); **Cancel**
  goes back to the saved layers. Saved layers survive a restart and the next rendering.

A job from before layers keeps its former cut setting: it goes to the layer of the same
name or to the only layer; otherwise the panel says so, to choose layer by layer.

Within a layer, a contour inside another (the hole of an “O”, a window) is cut before it,
so the piece does not move; at equal depth, the blade goes to the nearest contour (see
path sorting below).

On the machine, **One layer at a time** (System > Machines): for a cutter that cannot
change tool or setting during a cut, each layer becomes an output, the next one held
until **Resume**.

### Preflight

At rendering, the RIP flags (job warnings):
- a cut spot colour defined in RGB (define it in CMYK, spot colour);
- a cut path with a fill (a cut path is a stroke only);
- a path touching the page edge;
- a path covered by the artwork (put it on the top layer);
- an outlined stroke (a stroke turned into a shape: it would be cut twice);
- a stroke aligned inside or outside the path (align it to the centre);
- a path beyond the artboard (what lies outside is not cut);
- a cutter chosen with no cut spot colour recognised (the file's colours are listed);
- a path on a non-printing layer: it is not cut, unless **Also cut the paths of
  non-printing layers** is ticked (System > Cutting);
- open paths not cut, when open paths are not allowed.

### Cutter and cut after print

**Cutter** (send panel): the machine where the automatic cut goes. While it is not usable
(missing, offline, badly connected, or a layer with no setting on it), printing waits:
nothing goes out, so no ink or media is wasted. A cut waits until its page is 100 %
printed.

### Cut placement

**Cut placement** panel of the inspector. It turns the job for the cutter, not the file,
without rendering again:
- **Cut rotation** by quarter turns, registration frame included: turn the media by as
  much. The cut preview carries a grey arrow for the feed direction (the top of the page
  goes in first);
- **Expand the cut outwards (mm)**, from 0 to 5 mm: the piece grows by as much (a hole
  gets smaller);
- **Bowing compensation** (printed job): see below;
- in **cut only** mode only (without printing, the cut has no ink to follow): **Mirror**,
  **Scale X / Y (%)** (**Proportional** or not), **Copies X / Y** with **Gap X / Y (mm)**
  (100 per direction, 1000 in all), **Start from the machine origin**.

**Save cut placement** rebuilds the cut files. The expansion does not replace the
printer-cutter calibration (the machine's offset X / Y).

### Cutting tolerance

**Cutting tolerance** block of the send panel, **Add bleed near cuts**: ink that stops
within the **proximity threshold** of a closed contour is carried up to the contour, then
beyond it over the given **width** (0.1 to 10 mm): the cut leaves no white edge. The
composite preview does not show it, the plates do. Use cautious values; it does not
replace calibration.

### Open paths and sorting

In **System > Cutting** (on by default):
- **Allow open paths (banner vents)**: unticked, an open path is not cut, with a warning
  per page;
- **Sort the paths**: inner ones first, then nearest neighbour; unticked, paths are cut
  in creation order.

### Cut on arrival and after cutting

**Cut on arrival** (machine, System > Machines, ticked by default): unticked, each cut
arrives on hold in the machine's queue and goes when the operator clicks **Resume**.

**After cutting** (System > Cutting): **do nothing (history)** (default), **archive (can
be restored)**, or **remove from the queue**. An archived cut leaves the queue and goes
to its machine's **Archive**; **Restore** puts it back in the queue (on hold on a machine
without cut on arrival).

### Auto contour

**Auto contour** panel of each printed page of a PDF: the RIP analyses the rendered
image, removes the white around it and proposes a cut contour (`CutContour`). Settings:
**White threshold (1–255)**, **Remove those smaller than (mm²)**, **Smoothing (0–5)**,
**Outward offset (mm)**, **Remove inner contours**. **Preview** first (number of contours
and drawing), then **Accept**: the cut files are rebuilt. **Remove the auto contour**
takes it off. A new rendering erases it. Only on the rendering of a PDF.

### Bowing compensation

In **Cut placement**, **Compensate media bowing**: once the dryers are hot, the print
comes out bowed (middle behind the edges); the cut follows. Set the **Media width (mm)**
(the curve is centred on it), the **Start (mm of feed)**, counted from the top of the page
(nothing before), and the **Sag (mm)**. The curve sets in over 50 mm. Measure it on a
print.

### Machine variants

System > Machines, **Duplicate (other set of marks)** on a cutter: a variant, the same
physical machine, with its own **Set of marks** (or **— the job's —**). Connection, model
and cut settings are taken from the original machine at every save. A job assigned to the
variant gets its marks; its cut goes through the original machine, one output at a time.

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

**Automatic multi-frame**: with type 2, a page **1.5 m or longer** in the feed direction
takes the multi-segment frame (type 3) by itself, and its file the `MF` suffix. The Kona
manual (§ 6.3) advises against a single frame beyond 2 m and against a multi-frame under
1.5 m.

Types 2 and 3 follow the tables of the Kona application guide (ch. 4.4.2); the place of
the top code and the type 3 marker spacing are read from its drawings, without
dimensions (type 3 still to be proven).

Every frame carries an **▲ arrow** in its inner bottom-right corner: it points to the
edge that leaves the Epson first, the one fed first into the Kona (from the rear).

**Registration warnings**: an image under 25 cm per side is flagged (size advised for
Kona registration). The “under 5 cm of white” warning between image and marks is
suspended: our frames leave 10 mm, the rule is still to be proven on the machine.

**Background under marks** (send panel block), for dark, clear or reflective media:
**Print a background under the marks** lays a band of ink beyond each mark and barcode
(**Margin around the marks**, 0.5 to 13 mm), or, with **Flood the whole marks area (image
excluded)**, their whole area. **Ink**: white (`White` spot colour) or CMYK (values in %).
The black marks are printed over it with overprint: the background stays under them too.

**Sending to the Kona.** The RIP sends the cut as Mutoh CutServer does: with a frame it
declares two opposite frame corners (line centres), the Kona scans it, reads the code,
then cuts. **Cut settings** (speed in cm/s, force in g, acceleration in G) are declared
per machine in System > Machines, like CutServer's "cut layers"; the first is the
default. As in CutServer, each can also keep the **cutter defaults** (nothing sent),
force a **tool** (1 to 4), and follow a cut **pattern**: up to 10 "down/up" pairs in mm
(`8/2 8/2` = cut 8 mm, lift 2 mm), with **spread** (a whole number of patterns per
straight line, the cut starting and ending at sharp corners). Each cut layer goes out
with its own setting. **.plt file** with `?machine=kona` gives the file as the Kona
receives it, to send by hand (serial port 9600, 8N1, RTS/CTS). If the cut comes out
turned 180°, tick **cut turned** on the machine.

The printed page grows accordingly, and the cut file follows the same offset. The Kona
must be set to a resolution of **0.010 mm**.

## Presets and hot folders

**Save…** keeps the whole panel under a name (letters, digits, spaces, `-`, `_`, `+`).
Picking it in **Preset** fills the panel; **Delete** removes it.

Each preset has a **hot folder**: `/var/lib/eko-rip/entrees/<preset name>/` on the
server. A PDF, AI, SVG, TIFF, PNG or JPEG dropped there becomes a job with that preset once its copy is
complete (size stable for 5 s). A rejected file is moved to `refuses/`, with a
`.refus.txt` file giving the reason.

**Automatic folder**: `/var/lib/eko-rip/entrees-auto/` picks the preset itself, from the
**Automatic folder rules** (administrator, System). They are tried top to bottom (**↑**
moves a rule up), and the first that matches gives its preset:

- **Name pattern**: `*` for any characters, `?` for a single one, case ignored. `BACHE_*`
  takes `BACHE_client-12.pdf`, not `affiche_BACHE.pdf`.
- **Short side** and **long side** (mm, optional): the size of a PDF's first page, in
  either orientation. An empty bound means 0 or 100 m. A size rule never takes a file whose
  size is not read (image, SVG, PDF over 512 MB): put a rule without size after it.

Example: `BACHE_*` → Banner; `*.pdf`, long side from 2000 to empty → Large format; `*` →
Vinyl. A file no rule takes goes to `entrees-auto/refuses/` with its reason. The rules are
part of the configuration backup.

**Several renders at once**: the server renders several jobs at the same time (one per
core, 4 at most, set by `EKO_RIP_RENDUS`), shown in System > **Simultaneous renders**.
Arrivals from hot folders and from the ERP are queued in the order they were dropped, even
when a small file finishes rendering before a large one that came earlier. A file sent by
hand from the screen goes ahead of the automatic arrivals waiting for rendering, and is
queued as soon as it is ready.

**Tagged name pattern** (same editor, optional): the file name itself carries the preset
and more. With `{nom}__{machine}__{reglage}__{copies}`, `Affiche__epson__bache__30.pdf`
gives the Banner preset, the Epson and 30 copies. Each tag takes the text up to the
separator that follows it; the last one runs up to the extension. The double `__`
separator is advised: a single `_` often appears in names. Two tags must be separated, and
each is used once. Tag names are the French ones below.

| Tag | Effect |
|---|---|
| `{nom}` | Free text (name), no effect. |
| `{reglage}` | Saved preset, ignoring case, accents, spaces, `-` and `_` (`bache` = "Bâche"). |
| `{machine}` | Catalogue machine, by identifier or name: printer of the automatic queuing, or cutter of the job. |
| `{support}` | Epson media, by name. |
| `{copies}` | Job ticket quantity (1 to 100,000). |
| `{repetition}` | Prints of each page queued straight away (1 to 100). |
| `{laize}` | Roll width of the preset's tiling, in mm (`1600` or `1600mm`). |
| `{echelle}` | Scale in % (`200`, `12,5%`). |
| `{largeur}` / `{hauteur}` | Wanted width / height in mm, for a PDF: the scale follows (the smaller one when both are given). |

A name that does not follow the pattern, or a value that cannot be read, is not refused:
the job goes through the rules and carries a warning ("Name off the tagged pattern", "Name tag not
recognised"), also written to the server log. Without a recognised `{reglage}`, the rules pick
the preset; if none takes the file, the refusal reason lists these warnings.

Two boxes of the upload panel, kept in the preset, are mostly for hot folders:

- **One page per job**: a multi-page PDF becomes one job per page, named
  "file.pdf (p. 1/5)", "file.pdf (p. 2/5)"… with the same settings. No effect on a nest.
- **Refuse if a font is missing**: the file is refused (moved to `refuses/` for a hot
  folder) as soon as a font is not embedded, with the list of fonts. The 14 standard PDF
  fonts (Helvetica, Times, Courier, Symbol, ZapfDingbats) do not count: rendering swaps
  them for fonts of the same metrics. Without the box, the file goes through with a warning.

**Export, import** (administrator, System > Configuration backup): **Export presets**
downloads all saved presets, for another server; **Import presets** takes in those of an
export. **If the name exists**: **rename ("name 2")** or **overwrite**. Everything is
checked before writing.

## Tiling

**Tiling** block: tick **Split into panels when wider than the roll**, set the roll
width and the **overlap** (area printed on both neighbouring panels, for gluing or
welding). Panels start with the same width. Each panel becomes a page of the job, with
its own cut clipped to its edge. **Panel number and joining marks** adds a 10 mm band at
the foot of each panel: “Lé 2/3” and a vertical mark where the neighbouring panel stops
or starts, to line panels up when installing.

The block also sets the **Maximum tile height** (the page is then split into rows:
columns × rows), the **Weld** (overlap with no ink under the neighbour, at least 12.7 mm)
and the **Cut line in the middle of each overlap**. Unequal panels, tiles not to print,
a tile's own overlap and the print order are set in the viewer (see [Panels and
tiles](#panels-and-tiles)).

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

**Outline of the whole sheet (NestTrimBox layer)** adds a cut rectangle around the placed
items, in its own `NestTrimBox` layer, cut last and never printed. **Top margin (mm)** and
**Bottom margin (mm)** (0 to 200) leave blank above and below the items, for the cutting
table.

A page that carries cutter marks (Kona registration, frame…) but no cut path at all gets
the warning: *Cutter marks are printed, but the page has no cut path: ink and media for
nothing.*

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

## Nesting in the queue

Instead of leaving one by one, the prints waiting for the same printer can leave
together on a **sheet**: they are placed side by side across the roll width, and the
sheet leaves when its **trigger** is reached, or on **Print now**.

### Setting (System > Machines)

For each printer, under its settings (hover the rows for help):

- **Nesting** (strategy):
  - **One by one** (default): each print leaves alone, nothing is nested;
  - **Grouped in order**: side by side, by priority (nearest due date first), with no
    rotation or reordering;
  - **Media saving**: the whole sheet is recomputed at each arrival, tallest first, with
    the quarter turn when it shortens the sheet.
- **Triggers** (empty: no trigger; several: the first one reached):
  - **Start after (min)**: minutes since the **first** job of the sheet arrived;
  - **Start with no arrival for (min)**: minutes without a **new** job (counted from the
    last one in);
  - **Start at (m²)**: area of the placed jobs, copies included;
  - **Target length (mm)**: jobs that would make the sheet longer wait for the next one
    (badge *waits for the next sheet*), and a full sheet leaves by itself; a single job
    longer than the target still leaves, alone;
  - **Start at (% of target)**: the sheet leaves when it reaches this percentage of the
    target length (needs a target length);
  - **Start at length (mm)**: the sheet leaves as soon as it reaches this length; nothing
    is held back (the job that crosses it leaves with it);
  - **Full row leaves alone**: the job that would open a second row waits, and the full
    row leaves alone.

  With no trigger at all, the sheet waits for **Print now**.
- **Loading**: **Free (roll width)**, the sheet takes the width of the loaded roll (else
  the machine's), whatever the jobs' formats; **Strict (one sheet per format)**, jobs of
  different formats never share a sheet, and each sheet is laid on its format (capped by
  the roll). A job's format is the roll width of its media in the catalogue, else the one
  its tiling was made for; with no known format, the roll.
- **Space between copies (mm)** (5 by default): blank between two items side by side,
  and between rows; **Space between rows (mm)** replaces it between rows (empty: the
  same).
- **Justification**: **Left**, **Centre** or **Right**: where the group of items sits
  across the roll width. Centred or right, the sheet takes the whole roll width.
- **Origin offset X (mm)** (0 to 1000): blank on the left of the sheet, before anything
  else.
- **Purge strip**: **None**, **Left**, **Right** or **Both sides**; **Purge width (mm)**
  (1 to 100, 10 by default). The strip runs along the whole sheet on the chosen edge (on
  the left, after the origin offset), as four equal bands cyan, magenta, yellow, black,
  to clear each head; the space between copies separates it from the items.
- **Flip every other panel**: the even panels of a tiling (2, 4…) are laid upside down
  (half turn, same place), for a colour match between neighbouring panels.
- **Top margin (mm)** and **Bottom margin (mm)** (0 to 200): blank above and below the
  items, for the cutting table.
- **Outline of the whole sheet (NestTrimBox layer)**: cut rectangle around the placed
  items (without the offset or the purge), in the `NestTrimBox` layer, cut last, never
  printed.

### What nests together

Only jobs with the same **media**, the same **print mode**, the same **cutter** ("none"
included: a job to cut on the Kona and a job without a cut make two sheets) and the same
**mirror** share a sheet; and what the sheet has only once: the **resolution**, the
**registration**, the **colour** (profiles, intent), the **white**, the ink layers, the
**Background under marks**, the **Paths to cut** and the **Cutting tolerance**; finally
the **cut settings chosen per layer** (two jobs that choose two different settings for the
same layer do not nest together). A layer unticked on a job is not cut on the sheet. Only
PDFs nest; a page wider than the roll enters no sheet.

### Queue > nest in progress

In the **Queue** view, the column of a printer that nests shows "Nesting: *strategy*"
and, above its outputs, each **Nest in progress — n pages**:

- the sheet's media, cutter ("none") and mirror, and **NestTrimBox outline** when asked;
- the drawing of the imposition (the roll width, each item in place);
- **Length … mm · fill … %**: roll length used (margins included) and the share of that
  roll covered by the items; *Trigger reached: the sheet is leaving.* when it is;
- the list of pages: name, page, copies, order and due date; one tick box per page
  (**Print this page**): unticked, the page leaves the sheet and the queue until it is
  ticked again;
- **Print now**: the sheet leaves without waiting for its trigger;
- **Sheet PDF**: the exact PDF that will be printed (the preview is the PDF produced).

When the sheet leaves, it becomes a job "Imbrication *machine* — n page(s)", rendered then
queued on the printer (and on the group's cutter if it has one). Its ticket carries the
orders placed, the nearest due date and, in the notes, each page with its copies. The
placed prints (and the queued cut of those pages) become **Nested**.

**Abandoned sheet**: rendering failed, sheet job deleted, or the sheet's print cancelled,
failed or removed before being printed. Its jobs go back to the queue by themselves, in
their previous state (a held cut comes back held), and into the nest in progress.
Nothing comes back from a printed sheet. An abandoned sheet can no longer be redone:
**Redo** on its output, **Retry** on its job or queuing it again is refused (*Abandoned
sheet: its jobs went back to the queue…*), otherwise its jobs would print twice; **Print
now** on the nest in progress makes a correct new one. A printed sheet, on the other
hand, can be **Redone** (a wanted reprint).

## Colour

**Colour** block (Print mode):
- **Add an ICC profile…** uploads a `.icc` or `.icm` profile to the server (media
  profiles, Epson Wide Gamut input profile…).
- **CMYK input / RGB input**: profile applied to untagged data (or to all data, with
  **Ignore the PDF's profiles**).
- **Media (output)**: profile of the printed media; the preview simulates it.
- **Intent**: perceptual, relative colorimetric, saturation, absolute.

Spot colours that have their own ink (`White`, `Silver`…) are never converted. To adjust
a job's colours: the viewer's [colour filter](#colour-filter).

### Spot colour table

**System > Spot colours** (administrator): a spot colour in the file whose name is in the
table is printed in CMYK from **its Lab colour**, not from the fallback colour written in
the PDF. The conversion goes through the job's output profile and intent; the preview
shows the Lab colour. The job says so ("Spot colours printed in CMYK from the table").

- The name must be **identical**: case and spaces count ("PANTONE 485 C" does not match
  "Pantone 485 C"). An **alias** covers another spelling.
- **Import a Lab CSV**: your supplier's library (e.g. the HEXIS colour chart), one colour
  per line, `name;L;a;b` (`;`, tab or `,`; decimal comma allowed with `;`). A name already
  present refuses the whole import, nothing is written. No library ships with the RIP;
  only import the ones you are entitled to use.
- `White`, `Silver`, `Metallic` and cut colours cannot go into the table.
- **Spot colour intent**: relative colorimetric by default (aiming at the hue). It is set
  for the server in System > Spot colours; a setting or a job can pick another one (Colour
  block, "Spot colour intent"). Changing the server's one only affects jobs left on "The
  server's", at the next renderings. With Ghostscript's basic profiles, perceptual and
  relative give the same result; with a media ICC profile, they differ.
- A 50 % tint lays half of the full colour's CMYK recipe.
- A colour in the file that is neither an ink, nor a cut colour, nor in the table is
  flagged on the page ("spot colour missing from the table") and listed under **Spot
  colours met that are not in the table**: **Add** puts it in with its Lab value.
- Used by the next renderings: restart a job already rendered.

## System

**System** view: server version, job retention (30 days by default, set with
`EKO_RIP_CONSERVATION_JOURS`) and free space on the data disk, in red under 10 %. A
missing or damaged state file or folder is flagged there (“Files and folders”). The
**Cutting** block (administrator) holds the cut prefixes, non-printing layers, open
paths, path sorting and the **After cutting** choice (see [Cutting](#cutting)).

## Machines

**System > Machines**: the workshop's machine catalogue (one sheet per model: [help by machine](machines.md)). For each: an id (lowercase,
digits, `-`), a name, the model (Epson SC-S80600, Mutoh Kona, Graphtec FC7000, Roland
VersaCAMM VP), the roll width, the connection (serial or USB-serial, TCP network, device
such as `/dev/usb/lp0`, or folder), the SNMP address to read the loaded media, the cut
step (0.01 mm for the Kona, 0.025 mm by default elsewhere) and the cut **offset X / Y**
(−20 to 20 mm): the gap measured on a test cut between cut and print, added to every cut
sent to that machine. Also: the **Print origin X / Y** (Epson, direct driver), the
**Speed (m²/h)**, the **Pool**, and for a cutter its cut settings, **Cut turned (180°)**,
**One layer at a time**, **Cut on arrival** and **Duplicate (other set of marks)**, and
for a printer its **Nesting** (see [Nesting in the queue](#nesting-in-the-queue)).
**Save** applies at once: the print and cut buttons, the queue and the sidebar follow. A
machine with an output still to run cannot be removed.

- **Epson SC-S80600**: **Network (TCP)** connection, host = the printer's address, port
  9100 = direct driver; **Folder** = plates copied, nothing is printed. Fill in SNMP to
  stop the output on an alert.
- **Graphtec FC7000-75**: set the machine to HP-GL (Menu > Interface > Command), 0.025 mm
  step; ARMS registration is set on the machine's panel.
- **Roland VersaCAMM VP-300 / VP-540** (762 / 1371 mm): cutting goes over the network; to
  be tried at the workshop. Roland printing will follow a network capture.

**Printer pool**: the same **Pool** name on identical machines (same model, same roll
width). A **Print on pool "…"** button appears on each page; the output shows at every
member and goes to the first machine ready, which takes it. Refused if the known loaded
media of the members differ: load the same media, or duplicate the machine.

## Media and print modes

**System > Epson media** (administrator): the media catalogue. For each: **Name**, **No.
in the printer** (the media number stored in the Epson, which lets the loaded media be
recognised), **Roll width (mm)**, **Inks**, and its print **Modes**, written
`Name = 720 dpi / 8 passes; Other = 360 dpi / 4`. An optional third field gives the small,
medium and large droplet volume for the estimated ink: `Other = 360 dpi / 4 / 5-10-15 pL`.
Without it, 5-10-15 pL: starting values chosen by us (in proportion to the three dot
sizes of the screening), to calibrate. Prices stay in “Ink and media costs”, linked by
name. Each media and each mode keeps a stable identifier: renaming does not
change it.

Two more optional fields, in any order after the passes, set the mode's light-ink
separation (direct Epson driver): `Poster = 720 dpi / 8 / sharp / 260 %`.

- **Light-ink switch**: where the dark ink takes over from the light one (Lc, Lm, Lk).
  **soft**: full light ink up to 30 % density, long hand-over up to 70 %, some light ink
  kept under the dark; the least grain, for gradients and skin seen up close.
  **balanced** (default): light ink alone up to 24 %, hand-over up to 54 %. **sharp**:
  light ink capped at 60 %, hand-over from 18 to 33 %; less ink and faster drying, for
  displays seen from afar. Our values, to confirm on paper.
- **Total ink** (`260 %`, 100 to 400 %): the maximum sum of the 9 inks at one point
  (300 % without it).

**Proposed ink limit.** Print patches of increasing total ink (for instance 100 to 400 %
in 20 % steps), measure their L*, then send the series to
`POST /api/linearisation/limite-proposee` (`{"mesures": [[100, 58.0], [120, 47.2], …]}`).
The answer proposes the limit: the last step that still darkens by at least 1 L* per
extra 10 % of ink, rounded down to 5 %; a step that gets lighter (bronzing, bleeding)
stops it. Nothing is saved: read the returned gains, check the patch once dry, then enter
the limit in the mode.

**Linearisation chart**: `GET /api/linearisation/mire.cgats?n=21` (option
`encres=C,Lc,M`) gives a CGATS chart with one patch per ink and step, denser in the
highlights: 8 patches under 20 % out of 21, against 4 for an even ramp.

**When sending**, **Media and print mode**: the mode sets the rendering resolution; the
media links to the price list. **— resolution set by hand —** keeps the panel's
resolution. Changing mode on a received job renders again.

**Loaded media** (System > Media loaded in the Epson): recognised by its number read from
the printer, or chosen by the operator in the list (the choice wins until set back to
**— read from the printer —**). A number unknown to the catalogue is flagged. A print
waits until the loaded media is the job's (**Other media**), then leaves by itself; the
loaded roll's width bounds the page (**Too wide**). Unknown loaded media: nothing is
blocked.

**Media package**: **Export** (on a media row) gives one file with this media, its modes
and the presets that use it, without ICC profile. **Import a media package** takes it in
on another server: identifiers kept, re-import without duplicates; a package made by a
newer version is refused (update this server first).

## Media loaded in the Epson

**System > Media loaded in the Epson**: the width of the loaded roll, which the printer
measures when loading, is read over the network (SNMP, standard Printer-MIB) and becomes
the roll width of the Sheet and Tiling panels (**Use as roll width**, **Read again**).
Set the Epson's IP address in `EKO_RIP_EPSON_SNMP` (see the server installation guide).
Validated on the SC-S80600: the screen also shows the remaining roll length, the media
number, the heaters, the level of each ink, the alerts and the loaded media.

**Actual consumption**: the server reads the Epson every minute (inks and roll) and keeps
each change in `releves/epson.jsonl` (data folder). The screen shows what was used over 7
days; the API `GET /api/machines/epson/consommation?depuis=&jusqua=` (Unix seconds) gives it
for any period. Only decreases count: a new ink pack or roll is not counted as consumption.
The printer reports ink only to the nearest %: the figure is reliable over time, not for
a small job.

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

Two reports: **Export consumption (CSV)** gives the **real** figures, one row per print
in the log (see below); **Export estimates (CSV)** gives the **estimate**, one row per page
of each rendered job, printed or not: UTC date, job, page, media, area, media area,
estimated ml per ink (`BK_ml`, `C_ml`, `M_ml`, `Y_ml`, `WH_ml`, `MS_ml`, spot colours in
`autres_ml`), ink and media costs, total and VAT basis. Same conventions: `;` separator,
comma decimals, fixed columns, a `total` row equal to the sum, period through
`/api/rapports/estimations.csv?depuis=&jusqua=` (job creation date, Unix seconds).

Known limit: without a known roll width, the media counted is the page area, not the
roll length used across the whole roll width.

## Print log

**System > Print log**: each finished print, most recent first (the last 200): **Date**,
**Machine**, **Job** (and its page), **Media**, **Mode**, **Operator** (the one on the job
ticket), **Area**, **Length**, **Estimated ink**, **Real ink**, **Gap** and **Ink cost**.
Hovering the estimated or real ink gives the detail per ink (BK, C, LC…).

**Real ink**: through the direct driver, the server reads the Epson (SNMP) just before
sending and just after the print ends, on top of the reading every minute. What dropped
between the two is stored in the output; the queue shows it on the line of the finished
output: `Measured: ink 14 ml, media 574 mm`. The printer only gives ink to the nearest %
of a pouch (1 % of 700 ml = 7 ml):

- a print that sees no drop shows **< 7 ml**, never 0;
- a 1 % drop happened since the previous change of the same channel: it is shared
  between all the prints made since, in proportion to their estimated ink (failing that,
  their duration). A single figure stays approximate; the sum over a period is reliable.
  A drop outside any print (cleaning) is given to nobody; a new pouch is not consumption.

The pouch size is the cartridge entered in “Ink and media costs” (row of the ink, of its
plate or **All others**), 700 ml otherwise. Without an SNMP address, the real ink is
**not measured**.

**Length**: `369 → 574 mm (waste 204 mm)`: the page length, the length measured on the
roll between start and end, and the difference (lead-in, empty head, margins).
🏭 To be checked at the workshop on a real print (24/10: a 369.8 mm job, 574 mm measured,
i.e. 204 mm of waste).

**Estimated ink**: through the direct driver, the dots laid by the screening are counted
ink by ink and size by size; *ml = Σ dots × droplet volume of the mode* (see “Media and
print modes”), times the validated correction factor. A folder output (continuous-tone
plates, screened by other software) is **not computable**: no ink and no ink cost,
rather than a false figure. The estimate shown on the job card before printing remains
the one from the preview coverage.

**CSV reports**: **CSV report per print** (one row per print) and **CSV report per day**
(one row per day and machine), with a last `total` row equal to the sum of the rows.
`;` separator, comma decimals, columns always the same and in the same order: identity
(UTC date, machine, job, page, number of prints, media, mode, dpi, operator), area,
planned and real lengths, waste, estimation method (`points` or `non calculable`),
estimated and real ink side by side (totals, `sous_seuil`, gap in %, then an
`<ink>_estime_ml` / `<ink>_reel_ml` pair for each of the 11 catalogue inks), costs and
VAT basis. The address `/api/rapports/consommation.csv?depuis=&jusqua=` (Unix seconds)
limits it to a period.

**Retention (days)** (administrator): older prints leave the log, 90 days by default;
the purge runs every 5 minutes. **Purge the log** asks how many last days to keep (0:
erase all). Jobs and the queue are never touched. The printer readings stay.

**Estimated ink correction factors** (administrator): one factor per machine, resolution
and ink, 1 by default. The server proposes *real ink ÷ estimated ink* over the last 10
measured prints of each combination; **Apply** validates it, **Reset to 1** removes it.
It only applies to the following prints: the log keeps each one's factor. Retention and
factors are in the configuration backup.

## Not there yet

- White and silver through the direct Epson driver: they go out to a folder.
- Printing on the Roland.
- The Kona type 3 frame (multi-segment) has not been tried on the machine yet.
- A TIFF's spot colour channels: the value direction is still to check on a real
  Photoshop TIFF; transparency as a white plate will come with roadmap 9.12.
- The Epson swatch book's `MT-*` metallic colours are not converted yet.

## Glossary

The words of the screen, with their French equivalent: [glossary](glossary.md).
