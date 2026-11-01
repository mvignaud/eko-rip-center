# Client installation

The client is the only place to work: sending files, queue, settings. A browser
pointed at `http://<server>:8640` only shows a status page (version, machines, address to
enter in the client); the interface and the API are reserved to the client, recognised by
its `EKO-RIP-Client/` signature.

Download from the releases page:

| System | File |
|---|---|
| macOS (Apple Silicon and Intel) | `E-KO.RIP.Center_<version>_universal.dmg` |
| Windows | `E-KO.RIP.Center_<version>_x64-setup.exe` or `.msi` |
| Linux | `.deb` or `.AppImage` |

## First launch

The connection screen looks for E-KO RIP servers on the local network by itself and lists them
under **Servers found on the network** (name, address and version): one click connects.
**Search again** repeats the search (about 3 seconds). If no server shows up, enter its address
under **Enter an address**, for example `192.168.1.20:8640`. The client remembers the server
you chose.
To change it: **Server → Change server** menu (`Cmd/Ctrl + Shift + S`).

Discovery uses DNS-SD (mDNS, `_eko-rip._tcp`, UDP port 5353): it does not cross routers or VPNs
(use the address then). On Mac, macOS asks to allow local network access on first launch. The
server announces itself under the machine name (`EKO_RIP_CLOUD_NOM` if set); `EKO_RIP_ANNONCE=0`
turns the announcement off.

## First-launch warning

The Mac client published in the releases is signed (Developer ID) and notarized by Apple: it
opens without a warning. The Windows client is not signed yet; one approval per computer is
enough:
- **Windows**: “Windows protected your PC” → **More info** → **Run anyway**.
