# Epson SureColor SC-S80600

## What the RIP does

It renders the plates, separates them over the 9 inks (light and dark
pairs, ink limit), screens them and sends the print itself (direct driver, ESC/P2).

## Connect

Network cable, fixed IP.

| Connection | Effect |
|---|---|
| **Network (TCP)**, host = the Epson's IP, port **9100** | Direct driver: the RIP prints. |
| **IPP** to a **raw** CUPS queue | Same stream, through CUPS. |
| **Forward** to another E-KO RIP | Same stream, passed to the other server's Epson. |
| **Folder** | Nothing is printed: one file per channel (`K.tif`… `WH.tif`) and `envoi.json` in `sorties/<machine>/`. |

## In the RIP

**SNMP (media width)** = the Epson's IP (width and number of the loaded media,
inks, alerts), also in `EKO_RIP_EPSON_SNMP` for the loaded media screen; **Print origin
X / Y** set on a paper test; **Speed (m²/h)** measured in the workshop; media
catalogue and modes (System > Epson media), with the **media number** stored in the
printer.

## On the Epson

Store each media under a number and copy that number into the RIP's
catalogue; it is what makes the loaded media recognised.

## Check

`nc -vz <ip> 9100` from the server; System > Media loaded in the Epson should
show the roll width, the roll left and the inks.

## Common trouble

- *"Cannot send to the printer"*: a spot colour has no ink on the Epson (rename it `White`
  or `Metallic`, convert it to CMYK, or add it to the spot colour table), or the page has
  both white and silver. White and silver do not go through the direct driver yet: use a
  **Folder** machine meanwhile.
- *"The printer reports an alert: printing stopped"*: blocking alert read over SNMP (cover
  open…). Clear it, then **Redo**.
- *Output waiting "Other media"*: the loaded media is not the job's; load the right one,
  or pick the loaded media by hand.
- *No media width*: SNMP address, Epson on and on the same network, community
  (`EKO_RIP_EPSON_COMMUNAUTE`, `public` by default).

See: [direct Epson driver](../usage.md#direct-epson-driver),
[media and print modes](../usage.md#media-and-print-modes),
[loaded media](../usage.md#media-loaded-in-the-epson), [ink and cost](../usage.md#ink-and-cost).
