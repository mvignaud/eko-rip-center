---
titre: Fix an ERP bridge in error, replaced or in anomaly
description: Read the bridge status in System and fix it: signature refused, signature expired, ERP unreachable, replaced or anomaly.
sous_rubrique: rip-erp
position: 23
mots_cles: erp bridge, error, signature refused, signature expired, unreachable, replaced, anomaly, seq_manquant, 429
slug: rip-fix-erp-bridge-error
traduction_de: rip-resoudre-un-pont-erp-en-erreur
---

When the ERP bridge is not **Active** (*Actif*), the **System › ERP bridge** screen (*Système › Pont ERP*) says why. This article gives the steps to follow for each case.

## Before you start

You are the server administrator. Nothing is lost during an outage: everything goes out when it ends, in order.

## Steps

1. Open **Administration…**, the **System** view (*Système*), **ERP bridge** section (*Pont ERP*), and read **Status** (*État*) and **Last error** (*Dernière erreur*) (with its code).
2. If the status is **in error** (*en erreur*), follow the code:
   - signature refused: the RIP's secret is different from the ERP's; paste the same secret on both sides;
   - signature expired: the server clock is off by more than 5 minutes; check it with `timedatectl`;
   - ERP unreachable: check the network, the DNS and the ERP's certificate;
   - other error (401, 403, 404, 422, 429, 5xx): the code is shown as is; see [Understand ERP bridge exchanges](rip-comprendre-les-echanges-du-pont-erp.md).
3. If the status is **replaced** (*remplacé*), another installation is talking to the ERP in place of this one, or the sequence was reset from the ERP screen. If that is intended, do nothing. Otherwise, click **Draw a new instance** (*Tirer une nouvelle instance*) and confirm: the other installation will stop in turn.
4. If the status is **anomaly** (*anomalie*), the ERP and the RIP no longer count the same. Check with the ERP, then click **Draw a new instance**.
5. Check that **Last contact** (*Dernier contact*) is updating and that the number of acknowledged events is going up.

## Good to know

- In error, the RIP retries after 60 seconds, then every 5 minutes. It never skips a refused event.
- On `429` (too many requests), the RIP waits for the duration given by the ERP (1 hour at most), otherwise 60 seconds, before any new call.
- Drawing a new instance restarts the log from 1: the ERP takes this RIP for a new installation and replays everything.
- A sequence reset from the ERP screen looks like a replacement: the RIP stops and waits for you to draw a new instance.

## See also

- [Turn on the ERP bridge](rip-activer-le-pont-erp.md)
- [Understand ERP bridge instances](rip-comprendre-les-instances-du-pont-erp.md)
- [Verify the signature of ERP bridge events](rip-verifier-la-signature-des-evenements-du-pont-erp.md)
- [Monitor the server service](../rip-administration/rip-surveiller-le-service-du-serveur.md)
