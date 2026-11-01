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
- **E-mails**: sent as "E-KO RIP Cloud" `<no-reply@e-ko.fr>` over SMTP (STARTTLS or direct TLS,
  settings in Administration › E-mails or `EKO_CLOUD_SMTP_*`): forgotten password (1-hour
  single-use link), address confirmation at sign-up (required to link a server), "site offline for
  15 minutes" / "back online" alerts (at most one per site per hour), new connected app, and a
  Monday 8 a.m. (Paris) weekly summary; alerts and summary can be switched off in My account.
  Details in the French page.
- **Notifications**: the same events (offline, back online, new app) also go to a per-account
  journal polled by the desktop apps (`GET /api/moi/notifications?apres=<id>`, Bearer app token),
  to mobile apps through Expo push (`POST /api/moi/push`) and to browsers through Web Push
  (VAPID, `aes128gcm`, pure Rust, no OpenSSL; service worker at `/sw.js`, My account card). The
  apps ask for notification permission right after the first cloud sign-in. Details in the
  French page.
- **Tunnel**: a plain WebSocket (`Sec-WebSocket-Protocol: eko-tunnel/1`) carrying HTTP/2, so it
  goes through shared-hosting front ends (LiteSpeed, Passenger); the bare `Upgrade: eko-tunnel/1`
  of RIP 0.30 servers is still accepted.
- **Shared hosting (o2switch)**: `cloud/o2switch/app.js` is a dependency-free Node.js launcher
  for cPanel's "Setup Node.js App"; it starts the static (musl) relay binary and forwards HTTP
  and WebSocket traffic to it.

Full details (installation on a VPS or on o2switch, Caddy, protocol) in the French page: [docs/fr/cloud.md](../fr/cloud.md).
