# E-KO RIP Cloud (remote access)

The **E-KO RIP Cloud** platform (`https://platform.e-ko.rip`) gives access to a workshop RIP
server from outside, **without a VPN and without opening a port**: the RIP server itself opens an
outgoing connection (HTTPS, port 443) to the platform, and the platform forwards the requests of
workstations and the mobile app through it. Free service, open to any E-KO RIP Center server.

Status: **step 1** (tunnel and pairing). The portal (platform accounts, Google / Facebook, sites,
rights) comes in step 2; until then, remote users sign in with **the RIP server's accounts**.

- **Link a server**: the server needs at least one account. In **System › E-KO RIP Cloud**, an
  administrator clicks **Connect to the platform**; the server shows a short code (valid
  10 minutes), validated on the platform (`eko-rip-cloud valider <code>` for now, the portal
  later). The server then receives its key (`cloud.json`, mode 0600) and opens the tunnel. Remote
  address: `https://<site>.platform.e-ko.rip`, each site on its own origin: enter it as the
  server address in any client, or open it in a browser (the admin interface works there).
- **Never through the cloud** (`ferme_par_cloud`): ERP bridge routes, RIP-to-RIP forwarding,
  bridge settings and cloud settings. Without any account, every cloud request is refused
  (`cloud_sans_compte`).
- **Platform side**: site keys stored as SHA-256 only; only the RIP session cookie reaches a
  workshop (no `Authorization`, no portal cookies); each site has its own origin, cookie
  `Domain` attributes are stripped and portal cookies use the `__Host-` prefix. The legacy
  `/s/<site>/` form still works for API calls, with path-scoped cookies and a `sandbox` CSP.

Full details (installation, Caddy, protocol) in the French page: [docs/fr/cloud.md](../fr/cloud.md).
