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

Enter the server address, for example `192.168.1.20:8640`. The client remembers it.
To change it: **Server → Change server** menu (`Cmd/Ctrl + Shift + S`).

## First-launch warning

Packages are not signed yet.
- **macOS**: right-click the app → **Open**, then confirm.
- **Windows**: “Windows protected your PC” → **More info** → **Run anyway**.
