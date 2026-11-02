# E-KO RIP Cloud (remote access)

The **E-KO RIP Cloud** platform (`https://platform.e-ko.rip`) gives access to a workshop RIP
server from outside, **without a VPN and without opening a port**: the RIP server itself opens an
outgoing connection (HTTPS, port 443) to the platform, and the platform forwards the requests of
workstations and the mobile app through it. Open to any E-KO RIP Center server: the **Free** plan
(1 site, 1 printer, 1 cutter) has no time limit; paid plans add machines, sites and backup history.

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
- **Plans** (prices excluding VAT, unlimited users, yearly billing = 10 months for 12): Free
  (1 site, 1 printer + 1 cutter, latest backup, 1 GB); Per machine (€6/month per machine beyond the
  2 free ones, 30 backup versions, 5 GB per machine); Workshop (€19/month, 1 site, 5 machines,
  90 days of backups, 25 GB, +€4/month per extra machine); Pro (€49/month, 3 sites, 15 machines,
  1 year of backups, 100 GB, restore to another site, +€4/month per extra machine); Enterprise
  (quote). Machines are counted from the inventory each RIP server reports. Terms of sale at
  `/cgv`.
- **Subscription** page of the portal: subscribe (company details from the SIREN, billing
  contact, optional purchase order number, promo or partner code, terms of sale accepted), pay by
  **card** (secure field of the payment provider, 3-D Secure in an overlay, never leaving the
  portal) or **SEPA direct debit** (mandate shown in the page, signed with an SMS code; a mandate
  is valid for the company it was created for). 30-day trial when the account's first paid
  subscription is the Atelier plan (no invoice before it ends, the card is only checked; once
  per account, even after a cancellation). Invoices (PDF) at the start of each period; every e-mail comes from the
  platform. Plan changes are immediate (new period at the new price); cancellation takes effect
  at the end of the current period; failed payments get reminders around D+3, D+7 and D+14, then
  the account goes back to Free.
- **Over the plan**: owner and site administrators are warned (e-mail, notification, banner) and
  get **14 days** to upgrade or remove the extra sites or machines; then **remote access** to the
  affected sites is suspended (`402 offre_depassee`). Work inside the workshop is never stopped.
- **Online backups**: RIP servers send encrypted backups of their settings (nightly, after a
  change, or on demand), stored in France. **My sites › Backups** lists versions, storage used
  and quota; each version can be downloaded or restored on the server (Pro: to another site of
  the same owner). Retention: latest (Free), 30 versions (Per machine), 90 days (Workshop),
  1 year (Pro). Full quota or no nightly backup for two days: e-mail. After going back to Free,
  existing versions are kept 90 days.
- **Administration**: billing (billing tool API address and token, webhook secret, payment
  provider public key, plan codes per plan and cycle; webhook URL
  `https://platform.e-ko.rip/api/erp/webhooks`) and storage (S3 endpoint, region, bucket, keys,
  server-side encryption, separate bucket for the daily portal database copy) are set in the
  admin page only; secrets are never shown again. Each card has a **Test** button.
- **Tunnel**: a plain WebSocket (`Sec-WebSocket-Protocol: eko-tunnel/1`) carrying HTTP/2, so it
  goes through shared-hosting front ends (LiteSpeed, Passenger); the bare `Upgrade: eko-tunnel/1`
  of RIP 0.30 servers is still accepted.
- **Shared hosting (o2switch)**: `cloud/o2switch/app.js` is a dependency-free Node.js launcher
  for cPanel's "Setup Node.js App"; it starts the static (musl) relay binary and forwards HTTP
  and WebSocket traffic to it.

Full details (installation on a VPS or on o2switch, Caddy, protocol) in the French page: [docs/fr/cloud.md](../fr/cloud.md).

## Per-machine subscription and online backups

The plan is priced **per machine** (free: 1 site, 1 printer and 1 cutter). So that the
platform can count them, the RIP server reports its machines (id, name, model; printers on one
side, cutting plotters and cutting tables on the other) when the tunnel opens, after every change
of the machine catalogue and every 6 hours. A machine that prints (including the Roland
VersaCAMM, which cuts by itself) counts as a printer. The platform answers with the site's plan,
whether it includes backups, and whether the site is over its plan.

Paid plans include **online backups of the configuration**: the same archive as System › Backup
(machines, settings, ICC profiles, rules, templates, spot colours, prices, cutting…, no accounts
and no jobs), sent to the platform. The server pushes it:

- **every night**, at a random minute between 1:00 and 5:00 (server time), only if the
  configuration changed;
- **2 minutes after a configuration change**;
- **on demand**, by an administrator: `POST /api/cloud/sauvegarder`.

On a plan without backups the server stops trying until an inventory allows it again.
`GET /api/cloud` exposes `sauvegarde` (`derniere`: Unix time or `null`, `etat`: `ok`, `offre`,
`quota` or `erreur`, `message`) and `abonnement` (`offre`, `sauvegardes`, `depassement`).

**Restoring**: from the portal, the platform sends `POST /api/cloud/restaurer` `{"id": "…"}`
through the tunnel, signed (`X-EKO-Cloud-Service: t=<unix>,v1=<hex>`, HMAC-SHA256 keyed with the
SHA-256 of the site key, message `<t>.<body>`, 300 s tolerance, a signature works only once).
The server downloads the archive and applies it like `POST /api/sauvegarde`. Without a valid
signature: 401 `signature_cloud_invalide`, session or not. This is the only cloud-settings route
open through the tunnel.
