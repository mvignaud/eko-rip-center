---
titre: Understand the Epson direct driver
description: Learn what the direct driver does (9 inks, screening, SNMP alerts), its leading-edge margin and ink cap settings, and its current limits.
sous_rubrique: rip-machines
position: 45
mots_cles: direct driver, epson, 9 inks, snmp, leading margin, ink cap, origin, folder, white, silver
slug: rip-understand-epson-direct-driver
traduction_de: rip-comprendre-le-pilote-direct-de-l-epson
---

With the Epson SC-S80600 on a **Network (TCP)** connection, the RIP prints by itself. This article describes what this driver does, for the administrator and operators.

## Before you start

The Epson is connected and declared (see [Connect the Epson SureColor SC-S80600](rip-brancher-epson-sc-s80600.md)): printer **IP address** (*Adresse IP*) and **TCP port** (*Port TCP*) 9100.

## Steps

1. Check the machine's connection: **Network (TCP)** (*Réseau (TCP)*), port 9100. The RIP separates the render's C, M, Y, K plates across the machine's 9 inks (light and dark pairs, ink limit), screens them in 4 dot sizes and sends them band by band.
2. For printer alerts, fill in the machine's **SNMP** address: the status is read before sending, then every 5 s until the print ends.
3. Set the **Print origin X / Y (mm)** (*Origine X / Y impression (mm)*) using a paper test: X from the reference edge of the roll width (rounded to 16 dots), Y added to the leading margin. 0 by default.
4. Print a test and check the colours, the leading margin and the ink cap.

## Good to know

- A blocking alert (cover open…) stops the send: the output goes to *Failed* with "L'imprimante signale une alerte : impression arrêtée" ("The printer reports an alert: printing stopped"). Heating counts as printing in progress. Without SNMP, the output is finished as soon as everything has been sent.
- **Leading margin**: the image starts 20 mm after the start of the page, to avoid the dark band seen at the head of the first print.
- **Ink cap**: each channel is limited to 85%, against the thin bands seen in yellow on the first print.
- The first CMYK solid fills printed by this driver gave accurate colours.
- Leading margin and cap are to be confirmed in use; only first-test prints have been made: the other resolutions and real media remain to be proven in the workshop.
- **White and silver** do not yet go through the direct driver: queueing to an Epson on Network is refused ("Envoi à l'imprimante impossible", "Cannot send to the printer"). In the meantime, use a machine on **Folder** (*Dossier*).
- With a **Folder** connection, nothing is printed: the plates are copied to `/var/lib/eko-rip/sorties/epson/<job>-p<page>-<output>/` on the server, one file per printer channel (`K.tif`, `C.tif`, `M.tif`, `Y.tif`, then `WH.tif` or `MS.tif`) and `envoi.json` (channels in order, source plate, layers); the finished output shows **Déposée dans le dossier (pas imprimée)** ("Dropped in the folder (not printed)").
- Each print shows its estimated duration, and the column header shows the total remaining.

## See also

- [Connect the Epson SureColor SC-S80600](rip-brancher-epson-sc-s80600.md)
- [Fix a failed output to the machine](../rip-depannage/rip-sortie-vers-la-machine-echouee.md)
- [View the print log](../rip-couleur/rip-consulter-le-journal-des-impressions.md)
