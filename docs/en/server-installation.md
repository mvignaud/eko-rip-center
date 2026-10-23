# Server installation

The server runs on Linux. Printers and cutters are connected to it; workshop
computers reach it over the network.

Three ways to install it.

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

## Security

The server has no authentication yet. Keep it reachable from the workshop network
only: never expose port 8640 to the Internet.
