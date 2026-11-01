# Mutoh Kona 1650

## What the RIP does

It sends the cut like Mutoh CutServer (HP-GL at a 0.01 mm step), with
frame and barcode reading for registration.

## Connect

USB (FTDI serial adapter). The package's udev rule creates `/dev/kona` for the
`dialout` group. **Serial / USB-serial** connection, port `/dev/kona`, settings
`9600 cs8 -cstopb -parenb crtscts raw -echo` (9600 baud, 8N1, RTS/CTS).

## On the Kona

Resolution **0.010 mm**.

## In the RIP

Cut step **0.01 mm**; **cut settings** (speed, force, acceleration, tool,
pattern), the first one being the default; **Offset X / Y** measured on a test cut;
**Cut turned (180°)** if the cut comes out rotated by 180°.

## Registration

When sending, **Kona registration**; the **frame + barcode (type 2)** is
the one proven in the workshop. Feed into the Kona the edge shown by the frame's ▲ arrow.

## Check

`ls -l /dev/kona` once plugged in.

## Common trouble

- *No `/dev/kona`*: unplug, plug in again, then `journalctl -k | tail`.
- *Stops on "EPOS 6213"*: a type 1 frame without barcode sent directly; the Kona does not
  read it on its own. Use type 2.
- *Cut off target*: test cut, measure, enter the **Offset X / Y**.

See: [Kona registration](../usage.md#kona-registration), [cutting](../usage.md#cutting),
[variants](../usage.md#machine-variants).
