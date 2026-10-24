# Server installation

The server runs on Linux. Printers and cutters are connected to it; workshop
computers reach it over the network.

Four ways to install it.

## 0. Dedicated PC: installation image

> ⚠️ **Installing wipes the PC's whole disk.**

For an x86_64 PC used only for the RIP. Download
`eko-rip-center-serveur-<version>-amd64.iso` from the releases page and write it to a
USB stick (balenaEtcher, or `dd` on Linux and macOS).

1. Connect the PC to the workshop network (wired is best: the installer downloads
   Debian packages).
2. Boot from the stick and pick **Installer E-KO RIP - EFFACE LE DISQUE**.
3. Answer the only two questions: the `eko` account password (twice), then **Oui**
   (yes) to “Faut-il appliquer les changements sur les disques ?” (write the changes
   to disk?).
4. Remove the stick when the installer reboots the machine.

The PC boots into Debian 13 (French) with the server running. The console login
screen shows the address to open from workstations, for example
`http://192.168.1.20:8640`. Administration goes through SSH: `ssh eko@<address>`.

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
| `EKO_RIP_PLOTTER` | `/dev/kona` | cutter port |
| `EKO_RIP_PLOTTER_STTY` | `9600 cs8 -cstopb -parenb crtscts raw -echo` | cutter port settings |

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

The server has no authentication yet. Keep it reachable from the workshop network
only: never expose port 8640 to the Internet.
