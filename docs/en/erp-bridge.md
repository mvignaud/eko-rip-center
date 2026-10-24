# ERP ↔ RIP bridge — contract `eko-rip-erp/1` (lot R1)

What an ERP can rely on from E-KO RIP Center. Lot R1 lays the foundations: identifiers,
machine-to-machine access, event signature, state model. Later lots (R2 channel and
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

Event envelope (transport ships with lot R2):

```json
{
  "contrat": "eko-rip-erp/1",
  "seq": 42,
  "id": "6f1c0e8e-1b8e-4b9a-9d5e-2f6f3c1a7b10",
  "type": "sortie.etat",
  "le": 1793000000,
  "donnees": { }
}
```

- `seq`: increasing number **without gaps**, per server; the ERP may ask to replay
  "everything since N".
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

Coming (lot R5, direct Epson driver): `en_impression` and `imprimee`, reported by the
machine itself. Until then, `terminee` means "handed to the machine", not "out of it".

Suggested ERP mapping: job `termine` → "ready to print"; printer output `terminee` →
"printing"; cutter output `terminee` → "cut"; `echec` → "blocked", with the code.
