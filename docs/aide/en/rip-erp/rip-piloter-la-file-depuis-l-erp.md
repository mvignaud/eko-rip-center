---
titre: Drive the queue from the ERP
description: The actions the ERP can request on a project (suspend, resume, cancel, priority, deadline, operator, reprint) and their acknowledgement.
sous_rubrique: rip-erp
position: 31
mots_cles: actions, suspend, resume, cancel, priority, deadline, operator, reprint, travail.action, non_modifiables
slug: rip-drive-queue-from-erp
traduction_de: rip-piloter-la-file-depuis-l-erp
---

The ERP can act on the RIP's queue for a project: hold it, move it to the front, cancel it or reprint it. This article describes each action, its scope and the acknowledgement returned.

## Before you start

Actions target the jobs born from an ERP send (see [Receive the jobs sent by the ERP](rip-recevoir-les-travaux-envoyes-par-l-erp.md)).

## Steps

1. Send in `actions` an item `{ "id", "ref_erp", "action", "echeance", "operateur", "page" }`. `action` is one of: `suspendre`, `reprendre`, `annuler`, `priorite`, `echeance`, `operateur`, `reimprimer` (suspend, resume, cancel, priority, deadline, operator, reprint).
2. Provide what is required: `echeance` (Unix seconds) for `echeance`; `operateur` (text of 200 characters at most, empty = nobody) for `operateur`; `page` (from 1 to n) only for `reimprimer`.
3. Check the effect:
   - `suspendre`: waiting outputs become suspended; a job of the project that arrives afterwards goes out suspended;
   - `reprendre`: suspended outputs restart, except a cut whose print is not finished; lifts the hold set by `suspendre`;
   - `annuler`: the modifiable outputs are cancelled, and the project no longer goes into the queue; a job that has already been printed cannot be cancelled (code `deja_imprime`);
   - `priorite`: the outputs move to the front of the queue, in their order;
   - `echeance`: the deadline on the job ticket changes, without touching the queue;
   - `operateur`: the operator on the job ticket changes, without touching the queue; it appears in the RIP's reports;
   - `reimprimer`: every rendered job of the project goes back into the queue as when it arrived (same machine, cut included, `suspendre` hold respected), without re-rendering; `page` limits it to that page.
4. Read the `travail.action` acknowledgement: `{ "action_id", "ref_erp", "action", "code", "touchees", "non_modifiables" }`.

## Good to know

- **Scope**: an action targets all the jobs born from an ERP send that carry this `ref_erp`, and their outputs (print and cut) still in the queue. A job created at the RIP is never touched, even if it carries the same order on its job ticket.
- **Modifiable outputs**: waiting or suspended. An output that has started, finished, or been placed on a nesting sheet is left as it is and counted in `non_modifiables`.
- **Acknowledgement codes**: `null` if everything was done; `aucun_travail` if no ERP job carries this `ref_erp`; `deja_imprime`; `action_invalide` (malformed item, `detail` names the field); `rien_a_reimprimer` (no output added). `touchees` counts the outputs changed or added, or the jobs for `echeance` and `operateur`.
- `reimprimer` does not duplicate a job that is being rendered or that still has outputs in the queue: it is counted in `non_modifiables`. A cancelled project does not restart: send `reprendre` first.
- **Duplicates**: an action that has been taken is never taken again, even if proposed again; it is forgotten 30 days after the ERP stopped proposing it.

## See also

- [Receive the jobs sent by the ERP](rip-recevoir-les-travaux-envoyes-par-l-erp.md)
- [Manage a machine's queue](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
- [Understand job and output statuses](rip-comprendre-les-etats-des-travaux-et-des-sorties.md)
