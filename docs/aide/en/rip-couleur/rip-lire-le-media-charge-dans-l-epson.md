---
titre: Read the loaded media and actual consumption of the Epson
description: Read the roll width, media, inks and alerts of the Epson over the network, and track consumption over seven days.
sous_rubrique: rip-couleur
position: 47
mots_cles: loaded media, snmp, roll width, roll, inks, actual consumption, use as roll width, reread
slug: rip-read-the-loaded-media-and-actual-consumption-of-the-epson
traduction_de: rip-lire-le-media-charge-dans-l-epson
---

The RIP reads the Epson over the network to know which roll is loaded and to record consumption. This article is for the administrator.

## Before you start

The Epson is connected to the network (see [Connect the Epson SureColor SC-S80600](../rip-machines/rip-brancher-epson-sc-s80600.md)). Enter the Epson's address in `EKO_RIP_EPSON_SNMP` (file `/etc/default/eko-rip-serveur`) for this screen, then restart the service.

## Steps

1. Open **Administration…** (*Administration…*), **System** (*Système*) view, **Media loaded in the Epson** (*Média chargé dans l'Epson*) section.
2. Click **Reread** (*Relire*) to read the printer: the screen shows the roll width, the remaining length, the media number, the heaters, the level of each ink, the alerts and the loaded media.
3. Click **Use as roll width** (*Utiliser comme laize*) to take the measured width as the roll width of the Sheet and Tiling panels.
4. To change media by hand, choose it in the loaded media list; go back to **— lu sur l'imprimante —** (read from the printer) to resume reading.
5. Read the **actual consumption** over 7 days shown on screen.

## Good to know

- The roll width is read over the network (SNMP, standard Printer-MIB); validated on the SC-S80600.
- The server reads the Epson every minute (inks and roll) and keeps each change in `releves/epson.jsonl` (data folder).
- The API `GET /api/machines/epson/consommation?depuis=&jusqua=` (Unix seconds) gives the consumption for any period.
- Only drops count: a pouch or roll change is not counted as consumption.
- The printer reports ink only to the nearest 1%: the figure is reliable over time, not for a small job.
- No media width: check the SNMP address, that the Epson is on and on the same network, and the community (`EKO_RIP_EPSON_COMMUNAUTE`, `public` by default).

## See also

- [Set up a media and its print modes](rip-regler-un-support-et-ses-modes.md)
- [Consult the print log](rip-consulter-le-journal-des-impressions.md)
- [Connect the Epson SureColor SC-S80600](../rip-machines/rip-brancher-epson-sc-s80600.md)
