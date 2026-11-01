# Graphtec FC7000

## What the RIP does

It sends the cut in HP-GL, at a 0.025 mm step.

## Connect

Network (fixed IP) with a **Network (TCP)** connection, or serial / USB-serial.

## On the Graphtec

**HP-GL** commands (Menu > Interface > Command); **ARMS**
registration is set on the machine's panel.

## In the RIP

Step **0.025 mm**; cut settings; **Offset X / Y**.

## Check

`nc -vz <ip> <port>` from the server.

## Common trouble

- *Nothing cuts*: the interface is not set to HP-GL.
- *Output failed, machine unreachable*: check the address, the port and the cable.

See: [cutting](../usage.md#cutting), [machines](../usage.md#machines).
