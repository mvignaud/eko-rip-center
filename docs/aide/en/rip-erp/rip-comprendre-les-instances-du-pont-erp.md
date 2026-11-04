---
titre: Understand ERP bridge instances
description: Understand the instance and sequence number that guarantee no event is lost, duplicated or silently renumbered.
sous_rubrique: rip-erp
position: 24
mots_cles: instance, seq, sequence, replaced, anomaly, dernier_seq_traite, new instance, reset
slug: rip-understand-erp-bridge-instances
traduction_de: rip-comprendre-les-instances-du-pont-erp
---

Each installation of the bridge carries an **instance** and numbers its events with no gaps. This article explains how the ERP uses it and what happens when two RIPs compete for the same place.

## Before you start

It is for the RIP administrator and the ERP integrator. Setup is described in [Turn on the ERP bridge](rip-activer-le-pont-erp.md).

## Steps

1. Identify the instance: a UUID drawn at the server's first start, kept in `erp.json` and included in the configuration backup. It is shown in **System › ERP bridge** (*Système › Pont ERP*).
2. Remember that `seq` is an increasing number **with no gaps** for one instance: the ERP removes duplicates and detects gaps on the `(instance, seq)` pair.
3. Follow the ERP's response cases:
   - instance **never acknowledged**, `dernier_seq_traite = 0`: everything is replayed from `seq = 1` (new installation);
   - instance **already acknowledged**, response `0`: another installation has taken its place. The RIP no longer sends anything (neither reads nor sends) and shows "remplacé" ("replaced");
   - response **greater** than the RIP's last `seq` for the same instance, or replay requested for purged events: "anomalie" ("anomaly"), stop, never a silent renumbering;
   - instance unknown to the ERP that starts at `seq = 1`: it closes the previous one; with `seq` greater than 1, the ERP responds `409` `seq_manquant` and `dernier_seq_traite = 0`: the RIP replays from 1.
4. To restart a stopped RIP (replaced or anomaly), draw a new instance in **System › ERP bridge**, after checking with the ERP. The log restarts from 1.

## Good to know

- Only the administrator restarts a "replaced" RIP or one in "anomaly".
- **Sequence reset from the ERP screen**: the ERP then responds 0 for an already acknowledged instance, and the RIP cannot tell it from a replacement. It becomes "replaced" and stops; draw a new instance so it resumes (log replayed from 1).
- The ERP treats a `seq` it has already received as a duplicate: it responds 200 without redoing anything.
- A new instance replays the whole log history: warn the ERP before drawing a new instance.

## See also

- [Fix an ERP bridge in error, replaced or in anomaly](rip-resoudre-un-pont-erp-en-erreur.md)
- [Understand ERP bridge exchanges](rip-comprendre-les-echanges-du-pont-erp.md)
- [Verify the signature of ERP bridge events](rip-verifier-la-signature-des-evenements-du-pont-erp.md)
