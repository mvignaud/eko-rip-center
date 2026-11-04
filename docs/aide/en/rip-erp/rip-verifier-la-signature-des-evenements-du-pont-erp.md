---
titre: Verify the signature of ERP bridge events
description: Recompute the HMAC-SHA256 signature of a RIP event, with the test vector, so an ERP accepts the events and refuses replays.
sous_rubrique: rip-erp
position: 25
mots_cles: signature, hmac, sha256, x-eko-rip-signature, envelope, event, replay, test vector
slug: rip-verify-erp-bridge-event-signature
traduction_de: rip-verifier-la-signature-des-evenements-du-pont-erp
---

Every event the RIP sends to the ERP carries a signature. This article describes how the ERP verifies it and what an event's envelope contains. It is for the person who develops or checks the integration.

## Before you start

You have the shared secret (see [Create the ERP bridge token and secret](rip-creer-le-jeton-et-le-secret-du-pont-erp.md)), as displayed: 64 characters, taken as ASCII bytes.

## Steps

1. Read the event header: `X-EKO-RIP-Signature: t=<Unix seconds>,v1=<hex>`.
2. Take the **exact body** received, as bytes: do not re-serialise the JSON.
3. Build the message: `"<t>." + body`.
4. Compute `HMAC-SHA256(key, message)`, where the key is the secret as displayed, and write the result in lowercase hexadecimal.
5. Compare with `v1` in **constant time**. Refuse the event if the signature is wrong.
6. Also refuse a `t` that is more than 5 minutes from the ERP's clock (replay).
7. Treat a `seq` already seen as a duplicate: respond 200 without redoing anything.
8. Check your calculation with the test vector (RFC 4231, case 2): key `Jefe`, message `what do ya want for nothing?` gives `5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843`.

## Good to know

- Envelope of an event:

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
- `instance` identifies this installation of the bridge; `seq` is the increasing number with no gaps; `id` is the event's unique identifier; `le` is the time in Unix seconds; `type` is `catalogue.modifie`, `travail.etat`, `sortie.etat`, `travail.consommation`, `machine.etat`, `travail.refuse`, `travail.action` or `travail.estimation`.
- The RIP's read requests (`en-attente`, downloading a file) are signed the same way, over `"<t>.GET <path>?instance=<uuid>"`.
- The ERP ignores the fields and types it does not know.
- A signature refused on the ERP side appears on the RIP side as a bridge error: see [Fix an ERP bridge in error, replaced or in anomaly](rip-resoudre-un-pont-erp-en-erreur.md).

## See also

- [Create the ERP bridge token and secret](rip-creer-le-jeton-et-le-secret-du-pont-erp.md)
- [Understand ERP bridge instances](rip-comprendre-les-instances-du-pont-erp.md)
- [View the events sent to the ERP](rip-consulter-les-evenements-envoyes-a-l-erp.md)
