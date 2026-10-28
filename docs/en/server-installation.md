# Server installation

The server runs on Linux. Printers and cutters are connected to it; workshop
computers reach it over the network.

First: the [PC requirements](#server-pc-requirements). Four ways to install it. Then: [running the server](administration.md) (data,
backups, updates, FAQ).

## Server PC requirements

The server receives the files, renders them and talks to the machines: it is the computer
that must not stop. For each machine, see [help by machine](machines.md).

**System**: Ubuntu 24.04 LTS (or a recent Debian), 64-bit **x86 (amd64)** PC: the packages
and the image are built for that architecture.

**CPU and memory**: rendering (Ghostscript) is the heaviest part. Suggested starting point:
4 cores and 16 GB of memory; more for banners of several tens of m² at high resolution.
These figures are advice, not a measurement: the render time limit (`EKO_RIP_DELAI_S`,
30 min by default) quickly tells whether the computer is too small. For reference, the
workshop's test server runs on a Pentium G645T (2 cores, 2012), 16 GB of memory and a
640 GB spinning disk with no separate partition: it is enough for tests; it is a floor,
not a target.

**Dedicated SSD for the data**: an SSD reserved for the data folder (`/var/lib/eko-rip`),
separate from the system disk:

- a render weighs **≈ 1 GB per m² at 600 dpi** (a quarter at 300 dpi, four times more at
  1,200 dpi): a 1.6 × 10 m banner at 600 dpi is ≈ 16 GB;
- the server keeps `EKO_RIP_SEUIL_LIBRE_MO` free (5 GB by default): below it, it purges the
  oldest inactive jobs and refuses a job that would not fit;
- allow at least **500 GB**, more if jobs are kept long (`EKO_RIP_CONSERVATION_JOURS`);
  the Production view shows the queue's spool and how full the disk is.

A dedicated disk stops a large render from filling the system, and can be replaced or
enlarged without reinstalling. Mount it on `/var/lib/eko-rip` (or point `EKO_RIP_DATA` to
it) before the first start.

**UPS**: plug the server, the switch and, if possible, the router into a UPS. A power cut
stops the running render (the job fails as "interrupted", to retry) and an output being
sent (it fails, to redo: a cut does not resume halfway). State files survive the cut
(previous version kept, see [administration](administration.md#where-the-data-lives)), but
nothing replaces a clean shutdown: pick a UPS that lasts a few minutes and tells the
server to shut down (for instance with `nut`).

**Fixed IP addresses**: reserve them on the router (fixed DHCP lease) or set them by hand.

| Device | Why |
|---|---|
| RIP server | clients, shared folders (Samba), another RIP forwarding to it and the ERP reach it by its address |
| Epson | direct driver (port 9100) and SNMP reading (`EKO_RIP_EPSON_SNMP`) |
| Network cutters (Graphtec, Roland) | host entered in System > Machines |
| CUPS server, other E-KO RIP | host of the IPP and Forward connections |

**Network**: cable rather than Wi-Fi between the server and the machines. The server
listens on port **8640** (clients, forwarding), never to be opened to the Internet; it
reaches the machines (9100, 631 for IPP, SNMP on UDP 161) and, outbound over HTTPS, the
ERP and the APT repository. Samba (445) only if the hot folders are shared.

**USB ports**: one free port per USB cutter (Kona); the service is in the `dialout` group.

## 0. Dedicated PC: Ubuntu installation image

> ⚠️ **Installing wipes the PC's whole disk.**

The image is built from Ubuntu 24.04 LTS (about 4 GB). It downloads as a single file, with
its SHA-256 checksum, from the E-KO cloud:

| Version | Image | Checksum | Folder |
|---|---|---|---|
| 0.16.0 | [eko-rip-center-0.16.0-ubuntu-24.04-amd64.iso](https://nuage.e-ko.fr/public.php/dav/files/5fekYTc2pzXRor6/eko-rip-center-0.16.0-ubuntu-24.04-amd64.iso) | [.sha256](https://nuage.e-ko.fr/public.php/dav/files/5fekYTc2pzXRor6/eko-rip-center-0.16.0-ubuntu-24.04-amd64.iso.sha256) | [link](https://nuage.e-ko.fr/s/5fekYTc2pzXRor6) |

Newer server versions are then installed by updating the package (see
[running the server](administration.md)): the image is only for the first install.
Check the downloaded file:

- Linux or macOS: `shasum -a 256 -c eko-rip-center-0.16.0-ubuntu-24.04-amd64.iso.sha256`;
- Windows: `certutil -hashfile eko-rip-center-0.16.0-ubuntu-24.04-amd64.iso SHA256`, or in PowerShell
  `(Get-FileHash eko-rip-center-0.16.0-ubuntu-24.04-amd64.iso -Algorithm SHA256).Hash`, to compare with the `.sha256`.

Write the image to a USB stick (balenaEtcher, Rufus in **DD Image mode** — not ISO mode,
which rebuilds the boot and loses the profile menu —, or `dd`), connect the PC to the workshop
network (cable advised: the Ubuntu desktop is downloaded during installation) and boot
from the stick. The menu offers three profiles:

| Profile | For | Content |
|---|---|---|
| **Server + client on this PC** (default) | small workshop, a single PC | Ubuntu desktop, RIP server, E-KO RIP Center client opened at each session on this PC |
| **Server + Ubuntu desktop** | everyday PC that is also the server, clients elsewhere | Ubuntu desktop, RIP server |
| **Server only (no desktop)** | PC dedicated to the RIP | RIP server, SSH administration |

Both desktop profiles add:
- **CUPS**, to print on office and network printers (*Settings > Printers*);
- the **ArtCraft** suite (free editing apps: PhotoCraft, VectorCraft, PdfCraft,
  DesignCraft…, thirteen in all), downloaded during installation (≈ 1 GB) from its official
  releases `github.com/storytold/<app>`, each package's SHA-256 checked. If the download
  fails, the PC installation goes on; details in `/var/log/eko-artcraft.log`. To install or
  update later: `sudo eko-artcraft` (whole suite) or `sudo eko-artcraft vectorcraft pdfcraft`.

The installer only asks for the identity (your name, the PC name — `eko-rip` advised —,
the account name — `eko` advised — and its password), then confirms wiping the disk (“Continue with autoinstall?”: answer **yes**). Remove the stick
on reboot.

The PC starts with the server running. On the “server + client” profile, the client opens
and connects to the local server by itself. From other computers, the client connects to
the address shown on the login screen (e.g. `http://192.168.1.20:8640`). Administration:
`ssh eko@<address>`.

## 1. Debian package (recommended)

For Debian 13 (trixie) or Ubuntu 24.04 on x86_64. Download
`eko-rip-serveur_<version>_amd64.deb` from the releases page, then:

```bash
sudo apt install ./eko-rip-serveur_0.1.0-1_amd64.deb
```

The package installs Ghostscript, creates the `eko-rip` system user and starts the
service, which restarts with the machine.

```bash
systemctl status eko-rip-serveur
journalctl -u eko-rip-serveur -f
```

Jobs are stored in `/var/lib/eko-rip`. Settings live in
`/etc/default/eko-rip-serveur`; apply them with `sudo systemctl restart eko-rip-serveur`.

## 2. Docker

```bash
docker run -d --name eko-rip --restart unless-stopped \
  -p 8640:8640 -v eko-rip-donnees:/donnees \
  ghcr.io/mvignaud/eko-rip-center:0.1.0
```

The repository is private: `docker login ghcr.io` with a GitHub token that has
`read:packages`.

## 3. Plain binary

`eko-rip-serveur-<version>-linux-x86_64.tar.gz` holds the program. Install
Ghostscript (`apt install ghostscript`), then run `./eko-rip-serveur`.

## Settings

| Variable | Default | Purpose |
|---|---|---|
| `EKO_RIP_ADDR` | `0.0.0.0:8640` | listen address and port |
| `EKO_RIP_DATA` | `eko-rip-data` (package: `/var/lib/eko-rip`) | jobs folder |
| `EKO_RIP_GS` | `gs` | Ghostscript command |
| `EKO_RIP_MAX_MO` | `8192` | maximum size of a received file (PDF, SVG, TIFF, PNG, JPEG), in MB |
| `EKO_RIP_DELAI_S` | `1800` | maximum rendering time, in seconds |
| `EKO_RIP_RENDUS` | one per core, 4 at most | renders run at the same time (1 to 16); each takes a core and memory, lower it on a small computer |
| `EKO_RIP_CONSERVATION_JOURS` | `30` | jobs deleted after this many days (0: never) |
| `EKO_RIP_SEUIL_LIBRE_MO` | `5120` | free space to keep (MB): below it, the oldest inactive jobs are purged and a job that would not fit is refused (`espace_insuffisant`); 0: no threshold |
| `EKO_RIP_PLOTTER` | `/dev/kona` | cutter port |
| `EKO_RIP_PLOTTER_STTY` | `9600 cs8 -cstopb -parenb crtscts raw -echo` | cutter port settings |
| `EKO_RIP_EPSON_SNMP` | (empty) | Epson IP address: loaded media width read over SNMP |
| `EKO_RIP_EPSON_COMMUNAUTE` | `public` | Epson SNMP community |
| `EKO_RIP_ACCES_NAVIGATEUR` | `0` | `1`: full interface in a browser too (development only) |

## Sharing the hot folders (Samba)

To drop files from Mac and Windows workstations:

```bash
sudo apt install samba
```

Append to `/etc/samba/smb.conf`:

```ini
[eko-rip]
   path = /var/lib/eko-rip/entrees
   writable = yes
   force user = eko-rip
   valid users = eko
```

Then `sudo smbpasswd -a eko` and `sudo systemctl restart smbd`. Workstations open
`smb://<server>/eko-rip` (Mac) or `\\<server>\eko-rip` (Windows).

## Security

### Accounts

With no account declared, access stays open and the screen says so in red. To close
access, create at least one account on the server (the password is asked twice,
without echo, 10 characters minimum):

```bash
sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur utilisateur ajouter mathieu
```

Two roles:
- **administrator**: everything, including server settings (machine catalogue, ink and
  media prices, ICC profiles, saved presets);
- **operator**: sends, edits, prints, cuts, runs the queue and sheets; sees the machines
  and loaded media, but not the settings screens, which the server refuses anyway.

The role is the third argument (`admin` or `operateur`). Without it: administrator if
there is none yet, operator otherwise. Accounts created before 0.11 stay
administrators. Declaring an account again changes its role and password.

```bash
sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur utilisateur ajouter julie operateur
```

`utilisateur liste` lists accounts and their role; `utilisateur supprimer <name>` removes one (its
sessions end at once). No restart needed. Passwords are stored hashed with Argon2id in
`<data>/utilisateurs.json` (mode 0600).

A session lasts 12 hours (`HttpOnly`, `SameSite=Strict` cookie) and is lost when the
server restarts. After 5 wrong attempts for one name, that name is locked for 5
minutes. Hot folders need no account: control them through the share (Samba).

### Configuration backup

System > **Configuration backup** (administrator): **Download the backup** gives a JSON
file with the machine catalogue, prices, saved presets and uploaded ICC profiles.
**Restore…** replaces machines, prices and presets with the file's and adds its
profiles; everything is checked before the first write (a bad file changes nothing).
Accounts are not in it (recreate them on the server), nor jobs or the queue. For a new
server: install, create the administrator account, restore.

### Network

Traffic is not encrypted (HTTP): keep the server reachable from the workshop network
only. Never expose port 8640 to the Internet.
