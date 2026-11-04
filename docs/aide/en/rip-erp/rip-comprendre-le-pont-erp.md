---
titre: Understand the ERP bridge
description: The ERP bridge links the RIP to the E-KO ERP: what flows, in which direction, with no amounts and no port to open.
sous_rubrique: rip-erp
position: 20
mots_cles: erp bridge, erp, contract, eko-rip-erp, principles, hmac, https, contract version
slug: rip-understand-erp-bridge
traduction_de: rip-comprendre-le-pont-erp
---

The ERP bridge is the signed exchange between the RIP and the Suite E-KO ERP: the ERP sends orders and actions, and the RIP sends back the catalogue, the job and output statuses, and the real consumption. This article presents the principles before setup.

## Before you start

It is for the RIP administrator and the person who manages the ERP. For setup, see [Create the ERP bridge token and secret](rip-creer-le-jeton-et-le-secret-du-pont-erp.md) then [Turn on the ERP bridge](rip-activer-le-pont-erp.md).

## Steps

1. Remember the direction of the exchanges: the RIP **goes out** to the ERP over HTTPS. No port is opened at the workshop. The only routes the ERP calls on the RIP are those under the `/api/erp/` prefix, when the network allows it (same site, VPN).
2. Remember that **no amount travels**: the RIP sends physical quantities (areas, lengths, ink, time) and the ERP puts a value on them.
3. Identify the four families of exchanges:
   - the **catalogue** (machines, media, modes, saved settings, profiles) that the RIP publishes: see [View the events sent to the ERP](rip-consulter-les-evenements-envoyes-a-l-erp.md);
   - the **statuses** and **consumption** of jobs and outputs: see [Understand job and output statuses](rip-comprendre-les-etats-des-travaux-et-des-sorties.md);
   - the **jobs** that the ERP sends to the RIP: see [Receive the jobs sent by the ERP](rip-recevoir-les-travaux-envoyes-par-l-erp.md);
   - the **actions** and **estimates** requested by the ERP: see [Drive the queue from the ERP](rip-piloter-la-file-depuis-l-erp.md) and [Estimate a project from the ERP](rip-estimer-un-dossier-depuis-l-erp.md).
4. Remember that every event is signed and numbered: see [Verify the signature of ERP bridge events](rip-verifier-la-signature-des-evenements-du-pont-erp.md) and [Understand ERP bridge instances](rip-comprendre-les-instances-du-pont-erp.md).

## Good to know

- **Contract version**: it is called `eko-rip-erp/1`. It is returned by `GET /api/erp/contrat` and carried by every event. A compatible addition (field, event type, newly documented status) keeps `/1`: the ERP **ignores** the fields and types it does not know. An incompatible change moves to `/2`; the RIP announces it in its release log before delivering it.
- The ERP token and the signing secret are created on the RIP server, by an administrator: they are shown only once.
- The `/api/erp/` routes stay closed without a token, even when human access is open (server with no account).
- Errors have a JSON body `{ "code": "<stable code>", "detail": "<text or null>" }`.
- If the bridge screen shows **in error** (*en erreur*), **replaced** (*remplacé*) or **anomaly** (*anomalie*), see [Fix an ERP bridge in error, replaced or in anomaly](rip-resoudre-un-pont-erp-en-erreur.md).

## See also

- [Create the ERP bridge token and secret](rip-creer-le-jeton-et-le-secret-du-pont-erp.md)
- [Turn on the ERP bridge](rip-activer-le-pont-erp.md)
- [Understand ERP bridge exchanges](rip-comprendre-les-echanges-du-pont-erp.md)
- [Read the RIP glossary](../rip-premiers-pas/rip-consulter-le-glossaire.md)
