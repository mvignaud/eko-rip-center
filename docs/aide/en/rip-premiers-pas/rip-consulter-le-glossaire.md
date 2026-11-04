---
titre: Look up the RIP glossary
description: The words used on screen and in the help (job, plate, roll width, proof, registration, ERP bridge…) with their definition and their French interface label.
sous_rubrique: rip-premiers-pas
position: 25
mots_cles: glossary, definitions, vocabulary, plate, roll width, proof, registration, spot colour, french
slug: rip-look-up-the-rip-glossary
traduction_de: rip-consulter-le-glossaire
---

This glossary explains the words used on screen and in the help, with the French label shown in the software, whose interface is in French. Use it as a reference: browse it whenever a term is unfamiliar.

## Before you start

No prerequisites. The definitions are grouped in five families: jobs and queue, ink and colour, media and layout, cutting, machines and server.

## Steps

1. Find the family of the word you are looking for (job, colour, media, cutting, machine).
2. Look for the bold word in that family's table. A column gives its French label, useful when you read the software or the French help.
3. For the procedure that uses the word, follow the links in the **See also** section.

### Jobs and queue

| Term | In the French interface | Definition |
|---|---|---|
| **Job** | *Travail* | A received file (PDF, SVG, TIFF, PNG, JPEG) with its request: settings, pages, rendered plates, ticket. |
| **Rendering** | *Rendu* | Conversion of the file into ink plates (Ghostscript), done once on receipt; a print or a cut starts again without re-rendering. |
| **Preflight** | *Pré-vérification* | Check done on receipt: fonts not embedded, images with too low a resolution, cut paths, sizes. It warns, or rejects if the setting says so. |
| **Preset** | *Réglage* | A set of choices saved under a name (resolution, media, white, cutting, registration…), applied on sending or by a hot folder. |
| **Hot folder** | *Dossier surveillé* | A server folder where every dropped file becomes a job, with the folder's preset (`entrees/<preset>/`). |
| **Rule** | *Règle* | In `entrees-auto/`, picks a file's preset from its name or its page size. |
| **Tagged name pattern** | *Motif à balises* | In `entrees-auto/`, the shape of the file name (`{nom}__{machine}__{reglage}__{copies}`) that gives preset, machine, copies… |
| **Job ticket** | *Fiche travail* | Order number, customer, company, quantity, deadline, notes and operator of a job. |
| **Output** | *Sortie* | One page of a job sent to a machine (print or cut), in the queue. |
| **Queue** | *File d'attente* | The outputs to run, machine by machine, in order; a machine runs one output at a time. |
| **Waiting reason** | *Raison d'attente* | Why an output does not start (machine unreachable, other media, too wide…); the following ones go first. |
| **Spool** | *Tampon* | Weight of the files that the outputs still to run will send to the machines. |
| **History** | *Historique* | Finished, failed or cancelled outputs; **Redo** (*Refaire*) restarts without a new render. |
| **Proof** (approval proof) | *BAT* (bon à tirer) | A print to approve; on a second machine, the "-BAT" copy goes out and the original waits, suspended. |
| **Pool** | *Pool* | Identical machines grouped under one name: the output goes to the first one that is ready. |
| **Purge** | *Purge* | Deletion of old inactive jobs, after the retention period or below the free-space threshold. |

### Ink and colour

| Term | In the French interface | Definition |
|---|---|---|
| **Plate** | *Plaque* | 8-bit image of one ink (0 = full ink, 255 = no ink). |
| **Spot colour** | *Ton direct* | A named colour in the PDF (`White`, `Silver`, `CutContour`…), output on its own plate instead of being mixed into CMYK. |
| **Cut spot colour** | *Ton de découpe* | A spot colour that carries the cut paths (`CutContour`…); never printed. |
| **Spot colour table** | *Table des tons directs* | Exact name, aliases and Lab colour of each spot colour, to convert or recognise it. |
| **White** / **Silver** | *Blanc* / *Argent* | Special inks of the Epson, recognised by the spot colour name (`White`, `Metallic`…). |
| **Underlay** | *Sous-couche* | White laid under the ink, for a transparent or coloured media. |
| **Choke** | *Rétraction* | White pulled back from the edge of the ink, so that it does not stick out past the colour. |
| **ICC profile** | *Profil ICC* | Description of the colour of a media or a space, stored on the server. |
| **Rendering intent** | *Intention de rendu* | How an out-of-gamut colour is brought back into the media's gamut. |
| **Print mode** | *Mode d'impression* | Resolution, passes, heating, light-ink switch and total ink of a media. |
| **Light-ink switch** | *Bascule des claires* | Where the dark ink takes over from the light one (Lc, Lm, Lk): soft, balanced, sharp. |
| **Total ink** / **Ink limit** | *Encre totale* / *Limite d'encre* | Maximum sum of the inks at one point, in %. |
| **Linearisation** | *Linéarisation* | Measurement of ink ranges to adjust the response of each ink on a media. |
| **Screening** | *Tramage* | Conversion of ink values into dots (4 drop sizes for the Epson). |
| **Reading** | *Relevé* | Ink levels and roll read from the printer, for actual consumption. |

### Media and layout

| Term | In the French interface | Definition |
|---|---|---|
| **Media** | *Support* | The printed material (vinyl, banner…), with its roll width, inks and modes. |
| **Loaded media** | *Support chargé* | The media present in the machine, read on the Epson or chosen by the operator. |
| **Roll width** | *Laize* | Usable width of the roll. |
| **Bleed** | *Fond perdu* | Image extended beyond the finished size, so that a slightly shifted cut leaves no white. |
| **Grommet** | *Œillet* | Grommet mark placed around the edge of a banner. |
| **Tiling** | *Mosaïque* | A large visual cut into pieces that fit the roll width. |
| **Panel** | *Lé* | Vertical strip of a tiling, at most as wide as the roll width. |
| **Tile** | *Tuile* | One piece of a tiling in columns × rows; each tile is a page. |
| **Overlap** | *Recouvrement* | Part shared by two neighbouring panels, to join them at installation. |
| **Sheet** | *Planche* | Several jobs placed side by side on one page. |
| **Nesting** | *Imbrication* | Placing jobs on a sheet, by hand or automatically in the queue. |

### Cutting

| Term | In the French interface | Definition |
|---|---|---|
| **Path** | *Tracé* | A line the cutter follows. |
| **Cut layer** | *Couche de découpe* | The paths with the same spot colour name, cut together with the same cut setting. |
| **Cut setting** | *Réglage de coupe* | Speed, force, acceleration, tool and pattern of a layer. |
| **Registration** | *Repérage* | The cutter reads printed marks, to cut at the exact place of the print. |
| **Marks** / **Frame** | *Repères* / *Cadre* | Crosses or a frame (with a barcode for the Kona) printed around the image for registration. |
| **Auto contour** | *Détourage automatique* | A cut path computed around the image. |
| **Cutting tolerance** | *Tolérance de découpe* | Ink extended beyond the cut outlines, so that a slightly shifted cut leaves no white edge. |
| **Bowing compensation** | *Compensation de cintrage* | The cut follows the print that comes out of the dryers curved (middle lagging behind the edges). |
| **Offset X / Y** | *Décalage X / Y* | Measured gap between the cut and the print, added to every cut of a machine. |
| **Cut turned (180°)** | *Découpe retournée* | A cut rotated by 180°, when the media enters from the other end. |
| **Variant** | *Variante* | Copy of a cutter with its own set of marks. |

### Machines and server

| Term | In the French interface | Definition |
|---|---|---|
| **Machine** | *Machine* | A printer or cutter in the catalogue (System > Machines). |
| **Connection** | *Connexion* | How the server reaches the machine: TCP network, serial, device, folder, IPP or forwarding. |
| **Direct driver** | *Pilote direct* | The RIP builds the printer's data stream itself (ESC/P2 for the Epson) and sends it. |
| **Folder connection** | *Connexion Dossier* | Nothing is sent: the output is copied to `sorties/<machine>/`. |
| **IPP** | *IPP* | Network printing protocol (CUPS); the stream goes out raw in a Print-Job. |
| **Forwarding** | *Renvoi* | The stream goes to another E-KO RIP Center, which passes it to its machine. |
| **Token** | *Jeton* | Secret that opens machine-to-machine access (forwarding, ERP); never shown again. |
| **Speed** | *Cadence* | Printed area per hour (m²/h), to estimate durations. |
| **SNMP** | *SNMP* | Reading the Epson's status over the network (media, inks, alerts). |
| **ERP bridge** | *Pont ERP* | Signed exchanges between the RIP and the E-KO ERP (`eko-rip-erp/1` contract). |
| **Free space threshold** | *Seuil d'espace libre* | Room to keep on the data disk; below it, purge and refusal of large jobs. |
| **Client** | *Client* | Desktop application (Mac, Windows, Linux) that opens the server's screen. |

## Good to know

- The interface has a language menu that switches between French and English (the **English** button of the administration interface): the words in the "Term" column are the ones displayed then.
- A plate is the 8-bit image of one ink: 0 for full ink, 255 for no ink.
- A cut spot colour (`CutContour`…) is never printed: it only carries the cut paths.
- The **ERP bridge** is the signed exchange between the RIP and the E-KO ERP: see [Understand the ERP bridge](../rip-erp/rip-comprendre-le-pont-erp.md).

## See also

- [Send a job](../rip-travaux/rip-envoyer-un-travail.md)
- [Declare the spot colours](../rip-couleur/rip-declarer-les-tons-directs.md)
- [Set up a media and its modes](../rip-couleur/rip-regler-un-support-et-ses-modes.md)
- [Add a machine](../rip-machines/rip-ajouter-une-machine.md)
