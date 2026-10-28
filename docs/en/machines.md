# Help by machine

One sheet per model in the catalogue (System > Machines): what the RIP does with it, how to
connect it, what to set on the machine and in the RIP, how to check, and common trouble.
Each feature is described in [usage](usage.md), running the server in
[administration](administration.md), and the words in the [glossary](glossary.md).

For every machine:

- **Id**: lowercase letters, digits and `-` (`epson`, `kona`…). It names the
  `sorties/<machine>/` folder and is used when another RIP forwards to it: do not change it
  lightly.
- **One output at a time**, in the order of the machine's queue. A machine that still has
  an output to run cannot be removed from the catalogue.
- **Fixed IP address** for every network machine (see the
  [server PC requirements](server-installation.md#server-pc-requirements)).

## Epson SureColor SC-S80600 (print)

**What the RIP does**: it renders the plates, separates them over the 9 inks (light and dark
pairs, ink limit), screens them and sends the print itself (direct driver, ESC/P2).

**Connect**: network cable, fixed IP.

| Connection | Effect |
|---|---|
| **Network (TCP)**, host = the Epson's IP, port **9100** | Direct driver: the RIP prints. |
| **IPP** to a **raw** CUPS queue | Same stream, through CUPS. |
| **Forward** to another E-KO RIP | Same stream, passed to the other server's Epson. |
| **Folder** | Nothing is printed: one file per channel (`K.tif`… `WH.tif`) and `envoi.json` in `sorties/<machine>/`. |

**In the RIP**: **SNMP (media width)** = the Epson's IP (width and number of the loaded media,
inks, alerts), also in `EKO_RIP_EPSON_SNMP` for the loaded media screen; **Print origin
X / Y** set on a paper test; **Speed (m²/h)** measured in the workshop; media
catalogue and modes (System > Epson media), with the **media number** stored in the
printer.

**On the Epson**: store each media under a number and copy that number into the RIP's
catalogue; it is what makes the loaded media recognised.

**Check**: `nc -vz <ip> 9100` from the server; System > Media loaded in the Epson should
show the roll width, the roll left and the inks.

**Common trouble**

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

See: [direct Epson driver](usage.md#direct-epson-driver),
[media and print modes](usage.md#media-and-print-modes),
[loaded media](usage.md#media-loaded-in-the-epson), [ink and cost](usage.md#ink-and-cost).

## Mutoh Kona 1650 (cut)

**What the RIP does**: it sends the cut like Mutoh CutServer (HP-GL at a 0.01 mm step), with
frame and barcode reading for registration.

**Connect**: USB (FTDI serial adapter). The package's udev rule creates `/dev/kona` for the
`dialout` group. **Serial / USB-serial** connection, port `/dev/kona`, settings
`9600 cs8 -cstopb -parenb crtscts raw -echo` (9600 baud, 8N1, RTS/CTS).

**On the Kona**: resolution **0.010 mm**.

**In the RIP**: cut step **0.01 mm**; **cut settings** (speed, force, acceleration, tool,
pattern), the first one being the default; **Offset X / Y** measured on a test cut;
**Cut turned (180°)** if the cut comes out rotated by 180°.

**Registration**: when sending, **Kona registration**; the **frame + barcode (type 2)** is
the one proven in the workshop. Feed into the Kona the edge shown by the frame's ▲ arrow.

**Check**: `ls -l /dev/kona` once plugged in.

**Common trouble**

- *No `/dev/kona`*: unplug, plug in again, then `journalctl -k | tail`.
- *Stops on "EPOS 6213"*: a type 1 frame without barcode sent directly; the Kona does not
  read it on its own. Use type 2.
- *Cut off target*: test cut, measure, enter the **Offset X / Y**.

See: [Kona registration](usage.md#kona-registration), [cutting](usage.md#cutting),
[variants](usage.md#machine-variants).

## Graphtec FC7000 (cut)

**What the RIP does**: it sends the cut in HP-GL, at a 0.025 mm step.

**Connect**: network (fixed IP) with a **Network (TCP)** connection, or serial / USB-serial.

**On the Graphtec**: **HP-GL** commands (Menu > Interface > Command); **ARMS**
registration is set on the machine's panel.

**In the RIP**: step **0.025 mm**; cut settings; **Offset X / Y**.

**Check**: `nc -vz <ip> <port>` from the server.

**Common trouble**: *nothing cuts* → the interface is not set to HP-GL; *output failed,
machine unreachable* → address, port, cable.

See: [cutting](usage.md#cutting), [machines](usage.md#machines).

## Roland VersaCAMM VP-300 / VP-540 (cut)

**What the RIP does**: the cut goes over the network (roll width 762 or 1371 mm). Printing
through the Roland does not exist yet: it will come after a network capture. All of it is
still to be proven in the workshop.

**Connect**: network, fixed IP, **Network (TCP)** connection.

**Check**: `nc -vz <ip> <port>` from the server.

## Connections

| Connection | For | To enter | Check |
|---|---|---|---|
| **Network (TCP)** | Epson (direct driver), Graphtec, Roland | host (fixed IP), port (9100 for the Epson) | `nc -vz <ip> <port>` |
| **Serial / USB-serial** | Kona, cutter on USB-serial | port (`/dev/kona`), `stty` settings | `ls -l <port>`; the service is in `dialout` |
| **Device (USB…)** | machine as a USB printer | path (`/dev/usb/lp0`…) | `ls -l <path>` |
| **Folder** | no driver, third-party software watching the folder | nothing | `ls /var/lib/eko-rip/sorties/<machine>/` |
| **IPP** | **raw** CUPS queue, IPP printer | host, port 631, path (`/printers/<queue>`) | `lpstat -h <host> -p` |
| **Forward to another E-KO RIP** | machine plugged into another E-KO RIP | host, port 8640, the machine's id there, token | `eko-rip-serveur renvoi etat` on the other server |

IPP and forwarding in detail: [administration, machines](administration.md#machines).
