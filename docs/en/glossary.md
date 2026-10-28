# Glossary

The words of the screen and of the documentation, with their French equivalent (the one on
the French screen and in the [French documentation](../fr/glossaire.md)). Each feature is
described in [usage](usage.md).

## Jobs and queue

| Term | In French | Definition |
|---|---|---|
| **Job** | Travail | One received file (PDF, SVG, TIFF, PNG, JPEG) with its request: settings, pages, rendered plates, ticket. |
| **Rendering** | Rendu | Turning the file into ink plates (Ghostscript), done once on arrival; a print or a cut goes out again without rendering again. |
| **Preflight** | Pré-vérification | Check on arrival: fonts not embedded, low-resolution images, cut paths, sizes. It warns, or refuses when the preset says so. |
| **Preset** | Réglage | A named set of choices (resolution, media, white, cutting, registration…), applied when sending or by a hot folder. |
| **Hot folder** | Dossier surveillé | Server folder where every dropped file becomes a job, with the folder's preset (`entrees/<preset>/`). |
| **Rule** | Règle | In `entrees-auto/`, picks a file's preset from its name or page size. |
| **Job ticket** | Fiche travail | A job's order number, customer, company, quantity, due date, notes and operator. |
| **Output** | Sortie | One page of a job sent to a machine (print or cut), in the queue. |
| **Queue** | File d'attente | The outputs still to run, machine by machine, in order; a machine runs one output at a time. |
| **Waiting reason** | Raison d'attente | Why an output does not go (machine unreachable, other media, too wide…); the next ones go ahead of it. |
| **Spool** | Tampon | Size of the files the outputs still to run will send to the machines. |
| **History** | Historique | Finished, failed or cancelled outputs; **Redo** runs one again without rendering. |
| **Proof** | BAT (bon à tirer) | A copy to approve; on a second machine, the "-BAT" copy goes and the original waits, held. |
| **Pool** | Pool | Identical machines grouped under one name: the output goes to the first one ready. |
| **Purge** | Purge | Deleting old inactive jobs, after the retention time or below the free space threshold. |

## Ink and colour

| Term | In French | Definition |
|---|---|---|
| **Plate** | Plaque | 8-bit image of one ink (0 = full ink, 255 = no ink). |
| **Spot colour** | Ton direct | Named colour in the PDF (`White`, `Silver`, `CutContour`…), output on its own plate instead of being mixed into CMYK. |
| **Cut spot colour** | Ton de découpe | Spot colour carrying the cut paths (`CutContour`…); never printed. |
| **Spot colour table** | Table des tons directs | Exact name, aliases and Lab colour of each spot colour, to convert or recognise it. |
| **White** / **Silver (metallic)** | Blanc / Argent | The Epson's special inks, recognised by the spot colour name (`White`, `Metallic`…). |
| **Underlay** | Sous-couche | White laid under the ink, for clear or coloured media. |
| **Choke** | Rétraction | White pulled back from the ink edge, so it does not show past the colour. |
| **ICC profile** | Profil ICC | Colour description of a media or a colour space, uploaded to the server. |
| **Rendering intent** | Intention de rendu | How an out-of-gamut colour is brought into the media's gamut. |
| **Print mode** | Mode d'impression | Resolution, passes, heating, light-ink switch and total ink of a media. |
| **Light-ink switch** | Bascule des claires | Where the dark ink takes over from the light one (Lc, Lm, Lk): soft, balanced, sharp. |
| **Total ink limit** | Encre totale / Limite d'encre | Maximum sum of the inks at one point, in %. |
| **Linearisation** | Linéarisation | Measuring ink patches to set each ink's response on a media. |
| **Screening** | Tramage | Turning ink values into dots (4 drop sizes on the Epson). |
| **Reading** | Relevé | Ink levels and roll read from the printer, for the real consumption. |

## Media and layout

| Term | In French | Definition |
|---|---|---|
| **Media** | Support | The printed material (vinyl, banner…), with its roll width, inks and modes. |
| **Loaded media** | Support chargé | The media in the machine, read from the Epson or chosen by the operator. |
| **Roll width** | Laize | Usable width of the roll. |
| **Bleed** | Fond perdu | Image extended past the trim size, so a slightly off cut leaves no white. |
| **Grommet** | Œillet | Grommet mark placed around the edge of a banner. |
| **Tiling** | Mosaïque | A large visual split into pieces that fit the roll width. |
| **Panel** | Lé | Vertical strip of a tiled job, at most as wide as the roll. |
| **Tile** | Tuile | One piece of a tiling in columns × rows; each tile is a page. |
| **Overlap** | Recouvrement | Part shared by two neighbouring panels, to join them when fitting. |
| **Sheet** | Planche | Several jobs laid side by side on one page. |
| **Nesting** | Imbrication | Laying jobs on a sheet, by hand or automatically in the queue. |

## Cutting

| Term | In French | Definition |
|---|---|---|
| **Path** | Tracé | A line the cutter follows. |
| **Cut layer** | Couche de découpe | The paths of one spot colour name, cut together with one cut setting. |
| **Cut setting** | Réglage de coupe | Speed, force, acceleration, tool and pattern of a layer. |
| **Registration** | Repérage | The cutter reading printed marks, to cut exactly where the print is. |
| **Marks** / **Frame** | Repères / Cadre | Crosses or a frame (with a barcode for the Kona) printed around the image for registration. |
| **Auto contour** | Détourage automatique | Cut path computed around the image. |
| **Cutting tolerance** | Tolérance de découpe | Ink extended past the cut contours, so a slightly off cut leaves no white edge. |
| **Bowing compensation** | Compensation de cintrage | The cut follows a print that comes out of the dryers curved (middle behind the edges). |
| **Offset X / Y** | Décalage X / Y | Measured gap between cut and print, added to every cut on a machine. |
| **Cut turned (180°)** | Découpe retournée | Cut rotated by 180°, when the media goes in from the other end. |
| **Variant** | Variante | Copy of a cutter with its own set of marks. |

## Machines and server

| Term | In French | Definition |
|---|---|---|
| **Machine** | Machine | Printer or cutter in the catalogue (System > Machines). |
| **Connection** | Connexion | How the server reaches the machine: TCP network, serial, device, folder, IPP or forwarding. |
| **Direct driver** | Pilote direct | The RIP builds the printer's data stream itself (the Epson's ESC/P2) and sends it. |
| **Folder connection** | Connexion Dossier | Nothing is sent: the output is copied into `sorties/<machine>/`. |
| **IPP** | IPP | Network printing protocol (CUPS); the stream goes raw in a Print-Job. |
| **Forwarding** | Renvoi | The stream goes to another E-KO RIP Center, which passes it to its machine. |
| **Token** | Jeton | Secret opening a machine-to-machine access (forwarding, ERP); never shown again. |
| **Speed** | Cadence | Area printed per hour (m²/h), to estimate times. |
| **SNMP** | SNMP | Reading the Epson's state over the network (media, inks, alerts). |
| **ERP bridge** | Pont ERP | Signed exchanges between the RIP and the E-KO ERP (`eko-rip-erp/1` contract). |
| **Free space threshold** | Seuil d'espace libre | Space to keep on the data disk; below it, purge and refusal of large jobs. |
| **Client** | Client | Desktop application (Mac, Windows, Linux) that opens the server's screen. |
