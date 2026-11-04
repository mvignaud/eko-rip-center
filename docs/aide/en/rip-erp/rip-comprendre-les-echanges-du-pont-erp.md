---
titre: Understand ERP bridge exchanges
description: The event log, the call rhythm, the rate limit and the ERP responses that drive the RIP's retries.
sous_rubrique: rip-erp
position: 29
mots_cles: log, journal.jsonl, pending, events, rate, 429, retry-after, missing_seq, retry
slug: rip-understand-erp-bridge-exchanges
traduction_de: rip-comprendre-les-echanges-du-pont-erp
---

The RIP writes each event to a log before sending it, then talks to the ERP at regular intervals. This article describes that mechanism so you can understand why a send is late or repeated.

## Before you start

The bridge is turned on (see [Turn on the ERP bridge](rip-activer-le-pont-erp.md)). The terms instance and `seq` are explained in [Understand ERP bridge instances](rip-comprendre-les-instances-du-pont-erp.md).

## Steps

1. **Log.** While the bridge is on, each event is written to `pont/journal.jsonl` (one line, synced to disk) **before** any send. A power cut during the write leaves at most one cut-off line: it is removed on restart, and nothing else changes.
2. **Bridge state.** `pont/etat.json` keeps the last acknowledgement and the last published status of each job and each output.
3. **Request.** The RIP queries `GET <base>/en-attente?instance=<uuid>`, signed: every 30 seconds, every 5 seconds when a request is waiting (for example a catalogue request), and immediately when an event is added.
4. **Send.** It posts `POST <base>/evenements`: a list of at most 100 events, in `seq` order, signed over the exact body, of everything that follows the last acknowledgement.
5. **Response.** The response can contain `travaux` (jobs sent by the ERP), `actions` (cancel, suspend, resume, priority, deadline, operator, reprint) and `estimations` (estimates).

## Good to know

- **Rate**: at most one request per second (reads included), that is 60 per minute even in a burst, under the ERP's limit of 120. Events that arrived while waiting go out in the next batch.
- On `429`, the RIP waits for `Retry-After` (in seconds, 1 hour at most), otherwise 60 seconds, before any new call.
- `200` and `409 seq_manquant` return `dernier_seq_traite` ("last processed seq"): the RIP resends everything that follows. Any other error (401, 403, 404, 422, 429, 5xx, network) is shown in **System** (*Système*) with its code; the RIP retries after 60 seconds, then every 5 minutes. It never skips a refused event.
- Acknowledged events are purged after 30 days; those that are not acknowledged stay until they are, however long the outage.
- The older `annulations` field is no longer applied; it is flagged in the server log if it arrives non-empty: use `actions`.
- With the bridge off, nothing is logged. On activation, existing jobs and outputs are taken as they are.
- The `pont/` folder is not configuration: it is not backed up.

## See also

- [Fix an ERP bridge in error, replaced or in anomaly](rip-resoudre-un-pont-erp-en-erreur.md)
- [Receive the jobs sent by the ERP](rip-recevoir-les-travaux-envoyes-par-l-erp.md)
- [Drive the queue from the ERP](rip-piloter-la-file-depuis-l-erp.md)
- [Know where the server data is](../rip-administration/rip-savoir-ou-sont-les-donnees-du-serveur.md)
