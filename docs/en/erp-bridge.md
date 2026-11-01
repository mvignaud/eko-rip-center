# ERP ↔ RIP bridge — contract `eko-rip-erp/1` (lots R1 to R4)

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
  `travail.consommation`, `machine.etat`…

## 5. State model

Also published, machine-readable, in `GET /api/erp/contrat` (`etats`).

**Job** (rendering, the RIP step): `en_attente` received, waiting · `en_cours` rendering ·
`termine` rendered, ready to output · `echec` failed (stable error code). After a power
cut, an `en_cours` job becomes `echec` with code `interrompu`; it must be restarted (it
does not resume by itself).

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
- Rate: **at most one request per second** (GET included), i.e. 60 per minute even in a
  burst, below the ERP's limit of 120; events arriving meanwhile go in the next batch. On
  `429`, the RIP waits for `Retry-After` (in seconds, 1 h at most), else 60 s, before any
  new call.
- Answers: `200` and `409 seq_manquant` give `dernier_seq_traite`: the RIP resends
  everything after it. Any other error (401, 403, 404, 422, 429, 5xx, network) is shown in
  System with its code; the RIP retries after 60 s, then every 5 min. It never skips a
  refused event.
- `travaux`: jobs sent by the ERP (lot R3, § 7).
- `actions`: cancel, hold, resume, priority, due date, operator, reprint (lot R3.2 and roadmap 7.8, § 8). They replace the old
  `annulations` field, which is no longer applied (reported in the server log if non-empty).
- `estimations`: estimates requested by the ERP (lot R4, § 9).

### 6.4 Instances

- The instance (UUID v4) is drawn on the server's first start and kept in `erp.json`.
- Answer `dernier_seq_traite = 0` for an instance **never acknowledged**: everything is
  replayed from `seq = 1` (new installation).
- Answer `0` for an instance **already acknowledged**: another installation took over. The
  RIP sends nothing any more (no `GET`, no `POST`) and shows "replaced" in System. Only the
  administrator restarts it, by drawing a new instance: the log restarts from 1.
- Answer above the RIP's last `seq` (same instance), or replay asked for purged events:
  "anomaly", stop, never a silent renumbering. After checking with the ERP: new instance.
- **Sequence reset from the ERP screen**: the ERP then answers 0 for an instance already
  acknowledged, and the RIP cannot tell it from a replacement. It goes "replaced" and stops;
  draw a new instance in System to restart it (log replayed from 1).

### 6.5 Events

`catalogue.modifie` — full catalogue, when the bridge starts, on every change (machines, cut
layers, media, modes, saved settings, ICC profiles) and when the ERP asks. Keys: `machines`
(printers), `decoupeuses`, `couches_decoupe`, `supports`, `modes`, `prereglages`,
`profils`, and `serveur` (version). Each object carries its `uuid` (except profiles,
identified by `fichier`) and its `nom`; `machine_uuid`, `support_uuid`, `mode_uuid`,
`decoupeuse_uuid` link them. An unknown field is left out, never `null`. No price.

`travail.etat` — on each job state change:

```json
{ "travail_uuid": "…", "ref_erp": null, "etat": "termine", "code_erreur": null, "pages": 2 }
```

A job created from an ERP send (§ 7) also carries `envoi_id`, and its reference:

```json
{ "travail_uuid": "…", "ref_erp": "PROD-2026-00123", "envoi_id": "…", "etat": "en_attente", "code_erreur": null, "pages": 0 }
```

A job in a group (§ 7, linked jobs) also carries `groupe` and `partie`.

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
- `ref_erp`: the order reference for a job sent by the ERP (§ 7), copied into its outputs
  and consumption; `null` for a job created in the RIP.

`machine.etat` (roadmap 7.9) — state read from a printer (SNMP, every minute), published only
when it changes (the time alone does not count):

```json
{ "machine_uuid": "…", "machine": "epson", "encres": [{ "encre": "BK", "pourcent": 80 }],
  "longueur_restante_m": 37.5, "support_uuid": "…", "laize_mm": 1620,
  "chauffages": [{ "ecran_c": 40, "autre_c": 45 }], "alertes": ["printer cover open"],
  "bac_encre_usagee_pourcent": null, "releve_le": 1793000000 }
```

- `encres`: % left, to the point as the printer gives it; `longueur_restante_m`: length
  entered when the roll was loaded, counted down by the machine (`null` if unknown).
- `support_uuid` and `laize_mm`: loaded media (entered by the operator or read on the machine).
- `chauffages`: two values per heater; `ecran_c` is the one shown on the printer's screen.
- `alertes`: the machine's texts, as is, blocking or not.
- `bac_encre_usagee_pourcent`: `null` until it is read on the Epson.
- No on-demand send yet: the ERP keeps the last `machine.etat` received.

Media inventory and machine log (roadmap 7.8): no separate event. The inventory is the
`supports` of `catalogue.modifie` (what the RIP can print) and, machine by machine, the loaded
media and its remaining length from `machine.etat`. A machine's log is its `sortie.etat` (what
runs, when, failed or not), its `travail.consommation` (media, length, ink, duration) and its
`machine.etat` (alerts), all with `machine_uuid`.

## 7. Jobs sent by the ERP (lot R3)

Shape of a `travaux` item: § 11 of the ERP-side contract (in
eko-source) — `id` (send uuid), `ref_erp`, `nom`, `prereglage_uuid`, `copies`, `echeance`,
`fichier { chemin, nom, taille, sha256 }`, and for linked jobs `groupe` and `partie`
(roadmap 7.8).

- **Download**: on each `en-attente`, every send never taken is downloaded with
  `GET <base><fichier.chemin>`, signed like `en-attente`, at most 5 per cycle and 4 GB. The
  file is checked against `sha256` when the ERP gives it, then kept in
  `<data>/pont/recus/<id>/` until it becomes a job.
- **Creation**: the job takes the send's uuid, the saved setting named by
  `prereglage_uuid`, and its job sheet: order = `ref_erp`, quantity = `copies`, due date. A
  server stopped between creation and clean-up does not create a second job.
- **Linked jobs**: `groupe` (text, at most 200 characters) and `partie` (rank, 1 to 10,000)
  go together; one without the other is an `envoi_invalide`. A part's outputs are placed in
  the queue right after those of parts of lower or equal rank still queued, otherwise right
  before those of a higher rank: the group goes through as one block and in order, even if
  the renders finish out of order. A priority (§ 8) is still possible.
- **Acknowledgement**: the job's first `travail.etat` carries `envoi_id` (§ 6.5).
- **Refusal**: `travail.refuse` — `{ "envoi_id", "ref_erp", "code", "detail" }`. Codes:
  `envoi_invalide` (malformed item; `detail` names the field), `telechargement_impossible`
  (404), `empreinte_fausse`, `reglage_inconnu`, and job-creation refusals (`format_inconnu`,
  `pdf_illisible`, `polices_manquantes`…).
- **Duplicates**: a send taken or refused is never taken again, even if offered again; it is
  forgotten 30 days after the ERP stops offering it. An outage (network, 5xx, disk) is not a
  refusal: the send is retried on the next cycle.

## 8. ERP actions on the queue (lot R3.2)

Shape of an `actions` item: § 12 of the ERP-side contract — `{ "id", "ref_erp", "action",
"echeance", "operateur", "page" }`, `action` one of `suspendre`, `reprendre`, `annuler`,
`priorite`, `echeance`, `operateur`, `reimprimer`. `echeance` (Unix seconds) is required for
`echeance`, `operateur` (text, at most 200 characters, empty = nobody) for `operateur`;
`page` (1 to n) is allowed for `reimprimer` only.

- **Scope**: an action targets every job born from an ERP send that carries this `ref_erp`,
  and their outputs (print and cut) still in the queue. A job created on the RIP is never
  touched, even if its job sheet carries the same order.
- **Outputs that can change**: waiting or held. An output started, finished, or placed on a
  nested sheet is left as is and counted in `non_modifiables`.
- **Effect**:
  - `suspendre`: waiting outputs are held; a job of the order arriving later starts held.
  - `reprendre`: held outputs restart, except a cut whose print is not finished. Lifts the
    hold set by `suspendre`.
  - `annuler`: outputs that can change are cancelled, and the order no longer enters the
    queue. A job already printed is not cancelled: it is reported with `code` =
    `deja_imprime`.
  - `priorite`: outputs move to the head of the queue, in their order.
  - `echeance`: the job sheet's due date changes (the queue is untouched).
  - `operateur`: the job sheet's operator changes (the queue is untouched); it shows in the
    RIP's reports.
  - `reimprimer`: each rendered job of the order goes back to the queue as on arrival (same
    machine, cut included, `suspendre` hold respected), without rendering again; `page`
    limits it to that page. A job still rendering or with outputs still queued is not
    doubled: it is counted in `non_modifiables`. A cancelled order does not restart (send
    `reprendre` first).
- **Acknowledgement**: `travail.action` — `{ "action_id", "ref_erp", "action", "code",
  "touchees", "non_modifiables" }`. `code` is `null` if all went through, `aucun_travail` if
  no ERP job carries this `ref_erp`, `deja_imprime` (above), or `action_invalide` (malformed
  item; `detail` names the field), `rien_a_reimprimer` (no output added). `touchees` counts
  changed or added outputs, or jobs for `echeance` and `operateur`.
- **Duplicates**: like sends (§ 7) — an action taken is never taken again, even if offered
  again, and is forgotten 30 days after the ERP stops offering it.

## 9. Estimating an order (lot R4)

Shape of an `estimations` item: § 13 of the ERP-side contract — `{ "id", "ref_erp" }` for an
order already received, or the shape of a `travaux` item (§ 7: `prereglage_uuid`, `copies`,
`fichier`) for a file to estimate before sending (quote).

- **File to estimate (quote)**: downloaded like a send (signed, SHA-256 checked), rendered
  with the named setting in a hidden job — neither listed nor queued, even with automatic
  printing, no `travail.etat`, all its pages together — then estimated as below and
  deleted. A file that cannot be downloaded or rendered replies with `travail.estimation`
  carrying the refusal code (`empreinte_fausse`, `reglage_inconnu`, `pdf_illisible`,
  `rendu_echoue`…) and its `detail`, never with `travail.refuse`.

- **Scope**: the rendered jobs born from an ERP send carrying this `ref_erp` (§ 7). A failed
  job is left out; a job still rendering makes the request wait for the next round (30 s at
  most). Excluded pages do not count.
- **Figures**: those of each page's job sheet, multiplied by the job's copies (its quantity,
  at least one) — area, ink per plate (cut spot colours excluded), duration on the first
  printer of the catalogue — and the roll length of all copies nested together on that
  printer's loaded width, with its nesting setting and margins, without a target length.
- **Reply**: `travail.estimation` — `{ "estimation_id", "ref_erp", "code", "travaux",
  "pages", "copies", "surface_m2", "encres": [{ "nom", "ml" }], "ml",
  "duree_impression_min", "machine_uuid", "laize_mm", "longueur_mm", "les" }`.
  `longueur_mm` is `null` if a page does not fit the width; `duree_impression_min` is `null`
  without a known speed; `les` counts tiling panels, copies included.
- **No amounts**: no ink or media cost (decision D8 of the ERP, which sets its own prices).
- **Codes**: `null`, `aucun_travail` (no rendered job for this order, no figures),
  `estimation_invalide` (malformed item; `detail` names the field).
- **Duplicates**: like actions (§ 8).
