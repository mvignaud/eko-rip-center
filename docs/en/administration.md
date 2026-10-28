# Running the server

For whoever installs and looks after E-KO RIP Center in the workshop. Installation is in
[server installation](server-installation.md), daily use in [usage](usage.md).

## Day to day

```bash
systemctl status eko-rip-serveur
journalctl -u eko-rip-serveur -f
sudo systemctl restart eko-rip-serveur
```

The log records every failed render, every failed output (machine, job, page and
reason), and the "no user declared" warning while access is open. System, in the client,
shows the version, job retention and free space.

A restart stops a running render: the job fails as "interrupted", run it again with
**Retry**. Queued outputs stay in the queue. Sessions are lost: everyone signs in again.

## Where the data lives

Package: `/var/lib/eko-rip` (otherwise `EKO_RIP_DATA`).

| Path | Content |
|---|---|
| `machines.json` | machine catalogue |
| `tarifs.json` | ink and media prices, excl. or incl. VAT |
| `reglages.json` | saved presets |
| `erp.json` | ERP bridge: ERP token hash, HMAC secret, ERP URL, enable switch, instance (mode 0600) — see [erp-bridge.md](erp-bridge.md) |
| `pont/` | ERP bridge: event log (`journal.jsonl`) and sending state (`etat.json`); not configuration, not backed up |
| `supports.json` | media per machine and their print modes (stable identifiers); taken from `tarifs.json` on first start |
| `charges.json` | media loaded in each machine, set by the operator or read from the Epson |
| `tons_directs.json` | spot colour table (System > Spot colours: exact name, aliases, Lab colour); included in the backup and in the copy made before an update |
| `tons_inconnus.json` | spot colours met that are not in the table (number of jobs, last time), listed in System > Spot colours |
| `decoupe.json` | server cutting settings (System > Cutting: prefixes, non-printing layers, open paths, sorting, after cutting); included in the configuration backup and in the copy made before an update |
| `utilisateurs.json` | accounts (hashed passwords, mode 0600) |
| `file.json` | output queue and history |
| `profils/` | uploaded ICC profiles |
| `entrees/<preset>/` | hot folders, one per preset |
| `travaux/<id>/` | one folder per job: source, `job.json`, plates (`prod/`), previews |
| `sorties/<machine>/` | plates copied for a machine on a Folder connection (Epson), cut files of a cutter on Folder |
| `releves/<machine>.jsonl` | SNMP readings (inks, roll) for actual consumption |
| `sauvegardes/avant-<version>-<date>.tar.gz` | configuration copied before every package update (machines, media, presets, prices, accounts, ERP bridge, profiles; 10 kept, mode 0600) |
| `*.json.prec` | previous version of each state file, kept on every write |
| `*.json.illisible` | copy of a state file found damaged (power cut) |

**Power cut**: each state file is written to disk before it is put in place, and its
previous version is kept (`.prec`). If a file is found damaged, the server restores the
previous version and reports it in **System** ("State files"). If both are unreadable, the
damaged file is copied to `.illisible` and System says so: restore a backup. Damaged
accounts never open access: login is still required.

Jobs older than `EKO_RIP_CONSERVATION_JOURS` days (30 by default) are deleted with their
outputs; 0 keeps them.

## Backing up

- **Configuration** (machines, prices, presets, profiles): System > Configuration
  backup, in the client. Do it again after each settings change.
- **Everything, accounts included**: copy `/var/lib/eko-rip` with the service stopped,
  or at least `utilisateurs.json` on top of the configuration backup. Jobs are large
  (full-resolution plates) and can be rebuilt from their sources.

New server: install, create the administrator account, restore the configuration from
the client.

## Updating

Install the new version's package over the old one: data stays.

```bash
sudo apt install ./eko-rip-serveur_<version>-1_amd64.deb
```

The interface comes from the server: clients show new features as soon as the server is
updated. Updating a client only matters for the client itself (window, server
connection).

## Accounts

See [server installation](server-installation.md#accounts). Forgotten password: declare
the account again (`utilisateur ajouter <name> <role>`) to set a new one. No
administrator left: an account declared without a role becomes one.

## Machines

- **Kona (USB)**: the package's udev rule declares it to the FTDI driver and creates
  `/dev/kona` for the `dialout` group (the service's). Check: `ls -l /dev/kona` once
  plugged in. Missing: unplug, plug back, then `journalctl -k | tail`.
- **Network machine** (Graphtec, Roland): the address and port must answer from the
  server (`nc -vz <ip> <port>`).
- **Epson**: on a **Network (TCP)** connection, port 9100, the RIP prints through its
  direct driver (check `nc -vz <ip> 9100` from the server); on **Folder**, its plates go
  to `sorties/epson/`. The media width, state and alerts are read over SNMP when the
  address is set (the server speaks SNMP itself, nothing to install): without SNMP, the
  direct driver cannot stop a print on an alert.
- **IPP**: the machine's stream (the Epson's ESC/P2, a cutter's HP-GL) goes out as an IPP
  Print-Job, raw format. With CUPS, create a **raw** queue (no driver):
  `lpadmin -p epson -v socket://<ip>:9100 -E`, then path `/printers/epson`, port 631. The
  screen shows the full address (`ipp://host:631/printers/epson`). Check:
  `lpstat -h <host> -p`. The RIP waits for CUPS's answer: a stopped or missing queue fails
  the output with the IPP refusal spelled out.
- **Forward to another E-KO RIP**: the stream goes to another E-KO RIP Center server,
  which passes it as is to its machine (its id, e.g. `epson`), without queueing it again.
  On the receiving server, create the token (shown once; `retirer` closes forwarding,
  `etat` says whether it is open):

  ```bash
  sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur renvoi jeton
  ```

  then paste it into the **Forward** machine of the sending server (host, port 8640). The
  token is never shown again on screen; it only opens the forwarding route. The stream
  travels unencrypted on the local network: keep it between the shop's servers.

## ERP bridge

System > **ERP bridge** (administrator): base URL given by the ERP, HMAC secret
(`eko-rip-serveur erp secret`, or pasted; never shown again), enable switch. The RIP calls
the ERP over outgoing HTTPS: no port to open. If the screen says:

- **failing**: the code says what to do (signature refused → secret differs from the ERP's;
  signature expired → server clock, `timedatectl`; ERP unreachable → network, DNS,
  certificate). Nothing is lost: everything goes out when it is back, in order.
- **replaced**: another installation talks to the ERP instead of this one, or the sequence
  was reset from the ERP screen. If intended,
  nothing to do; otherwise, **Draw a new instance** (the other one will stop in turn).
- **anomaly**: the ERP and the RIP no longer count the same; check with the ERP, then draw a
  new instance.

Details: [erp-bridge.md](erp-bridge.md), § 6.

## FAQ

**The browser only shows a status page.** By design: the interface lives in the clients
(Mac, PC, Linux) only. A browser request to the API is refused (`client_requis`).

**"Open access" in red.** No account is declared: any client on the network can do
anything. Create an administrator account.

**"Reserved for the RIP administrator."** The account is an operator: machines, prices,
profiles, saved presets and backup are for the administrator.

**Too many attempts.** 5 wrong passwords: the name is locked for 5 minutes. Restarting
the service unlocks it (and ends every session).

**A render fails.** Ghostscript's message is in the error detail and the log. Usual
causes: protected or damaged PDF, `EKO_RIP_DELAI_S` exceeded on a very large file, full
disk (System). Missing fonts and low-resolution images are flagged on upload by the
preflight.

**Very large formats.** A page whose rendering would exceed 3 GB (for instance a 1.6 × 3 m
banner at 720 dpi) is rendered in horizontal bands, then its plates are joined back: it
takes longer, but there is nothing to set. A joined plate over 3.5 GB is written as
BigTIFF. Plan the space: a plate weighs up to width × height in dots, close to 4 GB per
ink for that banner, before compression.

**File too large.** Above `EKO_RIP_MAX_MO` (8192 MB by default). Raise it in
`/etc/default/eko-rip-serveur`, then restart.

**An output to a machine failed.** The reason is under the output, in the queue:
missing port (`/dev/kona`), machine off or unreachable, permission denied. Fix it, then
**Redo** from the history.

**A machine cannot be removed.** Not while an output waits for it (nor through a
restore): run or cancel it first.

**"Cannot send to the printer" on the Epson.** A spot colour has no ink on the Epson
(rename it `White` or `Metallic`, convert it to CMYK, or add it to the spot colour table
with its Lab colour, then restart the job), or the page carries both white
and silver. On a Network connection (direct driver), white and silver are refused for
now: use a machine on Folder.

**"The printer reports an alert: printing stopped."** The direct driver read a blocking
alert over SNMP (cover open…). Clear it on the Epson, then **Redo** the output from the
history.

**No media width from the Epson.** Check the machine's SNMP address, that the Epson is
on and on the same network, and the community (`EKO_RIP_EPSON_COMMUNAUTE`, `public` by
default).

**The cut is offset from the print.** Make a test cut, measure the gap, enter it as the
machine's **Offset X / Y** (System > Machines).

**The estimated time is off.** Measure the real speed (m² per hour, over the roll
width) and enter it as the machine's **Speed**.
