# ERP ↔ RIP bridge — contract `eko-rip-erp/1` (lots R1 and R2)

What an ERP can rely on from E-KO RIP Center. Lot R1 lays the foundations: identifiers,
machine-to-machine access, event signature, state model; lot R2 (§ 6) the channel: log,
sending, instances, catalogue, states and consumption. Later lots (R2 channel and
catalogue, R3 job intake, R4 estimate, R5 actual consumption) add to it without breaking it.

Principles:

- **No amount ever crosses**: the RIP sends physical quantities (areas, lengths, ink,
  time); the ERP puts prices on them.
- **The RIP calls out to the ERP** (HTTPS): no port is opened at the print shop. The only
  routes the ERP calls on the RIP are under `/api/erp/`, when the network allows it.

## 1. Contract version

- Name: `eko-rip-erp/1`, returned by `GET /api/erp/contrat` and carried by every event.
- A compatible addition (field, event type, documented new state) keeps `/1`. The ERP
  **ignores** unknown fields and types.
- An incompatible change moves to `/2`, announced in the changelog before release.

## 2. Identifiers

- Every **machine**, **cut layer**, **saved preset**, **media** and **print mode** has a
  `uuid` (UUID v4) **assigned by the server and immutable**.
- Display names and a machine's text id (`epson`, `kona`…) may change: rely on the `uuid`.
- An unknown `uuid` sent to the RIP is rejected (`identifiant_inconnu`).
- A renamed saved preset is another preset (new `uuid`): its name is also its hot folder.
- ICC profiles: identified by file name (no `uuid` yet).

## 3. Machine-to-machine access

- The ERP token is created on the RIP server by an administrator:

  ```bash
  sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur erp jeton
  ```

  It is shown **once**; the server keeps only its SHA-256 hash (`erp.json`, mode 0600).
  Creating a new one revokes the old one. `… erp etat` reports what is set.
- The ERP sends it on every call: `Authorization: Bearer <token>`.
- The token **only opens** `/api/erp/` routes:

  | Case | Response |
  |---|---|
  | `/api/erp/…` without token or wrong token | `401` `jeton_erp_requis` |
  | `/api/erp/…` with the right token | handled |
  | Token presented on another route | `403` `jeton_erp_hors_pont` |

  `/api/erp/` stays closed without a token **even when human access is open**.
- Errors: JSON body `{ "code": "<stable code>", "detail": "<text or null>" }`.

Lot R1 route: `GET /api/erp/contrat` → `{ contrat, serveur, signature, etats }`.

## 4. Event signature (RIP → ERP)

- The shared secret is created on the RIP server and shown once:

  ```bash
  sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur erp secret
  ```

- Every event carries the header:

  ```
  X-EKO-RIP-Signature: t=<Unix seconds>,v1=<hex>
  ```

  with `v1 = HMAC-SHA256(key, message)` in lowercase hex, where **key** = the secret as
  shown (64 characters, taken as ASCII bytes) and **message** = `"<t>." + body`, the body
  being the **exact bytes** received (do not re-serialise the JSON).
- The ERP **rejects** a wrong signature (constant-time compare) and a `t` more than 5
  minutes from its clock (replay). An already processed `seq` is a duplicate: answer 200,
  do nothing.
- Check vector (RFC 4231, case 2): key `Jefe`, message `what do ya want for nothing?` →
  `5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843`.

Event envelope (transport: § 6):

```json
{
  "contrat": "eko-rip-erp/1",
  "instance": "0c9d6a2e-5b1f-4f0e-9a7d-3e2b8c4f6a10",
  "seq": 42,
  "id": "6f1c0e8e-1b8e-4b9a-9d5e-2f6f3c1a7b10",
  "type": "sortie.etat",
  "le": 1793000000,
  "donnees": { }
}
```

- `instance`: identifier (UUID) of this bridge installation, drawn on first start, kept in
  `erp.json` and carried by the configuration backup (compatible addition, 25/10).
- `seq`: increasing number **without gaps** per instance; the ERP deduplicates and detects
  gaps on `(instance, seq)`. An unknown instance starting at `seq = 1` closes the previous one;
  an unknown instance with `seq > 1` gets 409 `seq_manquant` and `dernier_seq_traite = 0` (the
  RIP replays from 1). If the ERP answers, for the SAME instance, a `dernier_seq_traite` above
  the RIP's last `seq`, the RIP stops sending and reports it in System: it never renumbers
  silently.
- `id`: unique event id. `type`: `catalogue.modifie`, `travail.etat`, `sortie.etat`,
  `travail.consommation`…

## 5. State model

Also published, machine-readable, in `GET /api/erp/contrat` (`etats`).

**Job** (rendering, the RIP step): `en_attente` received, waiting · `en_cours` rendering ·
`termine` rendered, ready to output · `echec` failed (stable error code). After a power
cut, an `en_cours` job goes back to `en_attente` and rendering resumes.

**Output** (one page sent to one machine): `en_attente` queued · `suspendue` held by an
operator · `en_cours` being sent · `terminee` handed to the machine (print or cut) ·
`echec` failed (stable error code) · `annulee` removed before sending. After a power cut,
an `en_cours` output becomes `echec` with code `interrompu`; it does not restart by itself
(a cut cannot resume halfway).

`imbriquee` (compatible addition, lot R2): the output sits on a nesting sheet; the sheet, a
job of its own, prints it, and the output becomes `terminee` once the sheet is printed. The
sheet carries the consumption.

Coming (lot R5, direct Epson driver): `en_impression` and `imprimee`, reported by the
machine itself. Until then, `terminee` means "handed to the machine", not "out of it".

Suggested ERP mapping: job `termine` → "ready to print"; printer output `terminee` →
"printing"; cutter output `terminee` → "cut"; `echec` → "blocked", with the code.


## 6. Channel and events (lot R2, RIP side)

### 6.1 Settings

In the client, **System > ERP bridge** (administrator): ERP base URL, HMAC secret, enable.
The URL is the one shown by the ERP bridge screen
(`https://<host>/api/v1/gestion-imprimerie/rip/<workshop>`): `https` is required (`http` is
only accepted towards the machine itself, for tests). The secret is created with
`eko-rip-serveur erp secret` or pasted in the screen; it is **never** shown again nor
returned by the API, nor written into the configuration backup (which carries the URL, the
enable switch and the instance). The screen shows the state (disabled, active, failing,
replaced, anomaly), the last numbered `seq` and the last one acknowledged by the ERP, the
last contact, the last error, and the instance, with **Draw a new instance** (confirmation).

### 6.2 Log

- While the bridge is enabled, each event is written to `pont/journal.jsonl` (one line,
  synced to disk) **before** anything is sent; `seq` grows without gaps for the instance. A
  power cut during the write leaves at most one cut line: it is removed at restart, nothing
  else changes (no event lost, none renumbered).
- The bridge state (`pont/etat.json`) keeps the last acknowledgement and the last published
  state of each job and output.
- Acknowledged events are purged after 30 days; the others stay until acknowledged, however
  long the outage.
- Bridge disabled: nothing is logged. When enabled, existing jobs and outputs are taken as
  they are; only their next changes make an event.

### 6.3 Exchanges

- `GET <base>/en-attente?instance=<uuid>`, always with the instance, signed over
  `"<t>.GET <path>?instance=<uuid>"`; every 30 s, every 5 s while a request is pending
  (`catalogue_demande`), and as soon as an event is added.
- Then `POST <base>/evenements`: a **list** of at most 100 events, in `seq` order, signed
  over the exact body; everything after the last acknowledgement.
- Answers: `200` and `409 seq_manquant` give `dernier_seq_traite`: the RIP resends
  everything after it. Any other error (401, 403, 404, 422, 429, 5xx, network) is shown in
  System with its code; the RIP retries after 60 s, then every 5 min. It never skips a
  refused event.
- `travaux`, `annulations`, `estimations`: read, ignored until their shape is set (lots R3
  to R5); the server log says so ("non prises en charge").

### 6.4 Instances

- The instance (UUID v4) is drawn on the server's first start and kept in `erp.json`.
- Answer `dernier_seq_traite = 0` for an instance **never acknowledged**: everything is
  replayed from `seq = 1` (new installation).
- Answer `0` for an instance **already acknowledged**: another installation took over. The
  RIP sends nothing any more (no `GET`, no `POST`) and shows "replaced" in System. Only the
  administrator restarts it, by drawing a new instance: the log restarts from 1.
- Answer above the RIP's last `seq` (same instance), or replay asked for purged events:
  "anomaly", stop, never a silent renumbering. After checking with the ERP: new instance.

### 6.5 Events

`catalogue.modifie` — full catalogue, when the bridge starts, on every change (machines, cut
layers, media, modes, saved settings, ICC profiles) and when the ERP asks. Keys: `machines`
(printers), `decoupeuses`, `couches_decoupe`, `supports`, `modes`, `prereglages`,
`profils`, and `serveur` (version). Each object carries its `uuid` (except profiles,
identified by `fichier`) and its `nom`; `machine_uuid`, `support_uuid`, `mode_uuid`,
`decoupeuse_uuid` link them. No price.

`travail.etat` — on each job state change:

```json
{ "travail_uuid": "…", "ref_erp": null, "etat": "termine", "code_erreur": null, "pages": 2 }
```

`sortie.etat` — on each output state change:

```json
{ "sortie_uuid": "…", "travail_uuid": "…", "ref_erp": null, "page": 1, "machine_uuid": "…",
  "machine": "epson", "operation": "impression", "etat": "en_attente", "code_erreur": null,
  "raison_attente": "attente_support", "planche_uuid": null }
```

`travail.consommation` — at the end of a print, then of a cut (output `terminee`):

```json
{ "travail_uuid": "…", "sortie_uuid": "…", "ref_erp": null, "page": 1, "operation": "impression",
  "machine_uuid": "…", "machine": "epson", "support_uuid": "…", "mode_uuid": "…", "couche": null,
  "copies": 1, "surface_m2": 1.2, "longueur_prevue_mm": 1000, "longueur_mm": 1204,
  "longueur_source": "reel", "gache_mm": 204, "encre_ml": { "C": 3.5 }, "encre_sous_seuil_ml": null,
  "encre_ml_estimee": { "C": 3.1 }, "encre_source": "reel", "duree_s": 600, "debut": 1793000000, "fin": 1793000600 }
```

- Nothing is made up: without a machine reading (SNMP), `encre_ml`, `longueur_mm` and
  `gache_mm` are `null`; `encre_source` tells where the given ink comes from (`reel`
  measured, `estime` from the dots laid by the direct driver, `null`: none).
- `encre_ml` is the real ink split at the end time (splitting a drop among several prints
  may refine it later in the RIP reports).
- `ref_erp` is `null` until jobs are received from the ERP (lot R3).
