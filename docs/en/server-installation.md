# Server installation

The server runs on Linux. Printers and cutters are connected to it; workshop
computers reach it over the network.

Four ways to install it. Then: [running the server](administration.md) (data,
backups, updates, FAQ).

## 0. Dedicated PC: Ubuntu installation image

> ⚠️ **Installing wipes the PC's whole disk.**

The image is built from Ubuntu 24.04 LTS (about 4 GB). A GitHub release refuses files over
2 GB: it is published in parts (`.part0`, `.part1`, `.part2`…) with a `.sha256` file.
Download every part and the `.sha256` into one folder, then join them in order:

- Linux or macOS: `cat <name>.iso.part* > <name>.iso`, then `shasum -a 256 -c <name>.iso.sha256`;
- Windows: `copy /b <name>.iso.part0 + <name>.iso.part1 + <name>.iso.part2 <name>.iso` (as many
  parts as published), then `certutil -hashfile <name>.iso SHA256` to compare with the `.sha256`.
  In **PowerShell**, `copy` is another command: use `cmd /c "copy /b …"`, and check with
  `(Get-FileHash <name>.iso -Algorithm SHA256).Hash`.

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
| `EKO_RIP_MAX_MO` | `2048` | maximum PDF size, in MB |
| `EKO_RIP_DELAI_S` | `1800` | maximum rendering time, in seconds |
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
