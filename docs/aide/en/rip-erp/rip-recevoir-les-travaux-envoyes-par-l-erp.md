---
titre: Receive the jobs sent by the ERP
description: Understand how the RIP downloads a project's file from the ERP, creates the job, groups linked jobs and refuses an invalid send.
sous_rubrique: rip-erp
position: 30
mots_cles: erp jobs, send, ref_erp, prereglage_uuid, sha256, group, part, travail.refuse, empreinte_fausse
slug: rip-receive-jobs-sent-by-erp
traduction_de: rip-recevoir-les-travaux-envoyes-par-l-erp
---

The ERP can drop a file into the RIP: the job is created automatically with the designated saved setting. This article describes the send, the job creation, linked jobs and refusals.

## Before you start

The bridge is turned on. The ERP must know the `uuid` of the saved setting to use (see [Understand ERP bridge identifiers](rip-comprendre-les-identifiants-du-pont-erp.md)).

## Steps

1. **Send**: the ERP places in `travaux` an item with `id` (uuid of the send), `ref_erp`, `nom` (name), `prereglage_uuid`, `copies`, `echeance` (deadline), `fichier { chemin, nom, taille, sha256 }` and, for linked jobs, `groupe` and `partie`.
2. **Download**: on each request, every send never taken is downloaded by `GET <base><fichier.chemin>`, signed like the request. At most five sends per cycle, at most 4 GB per file. The file is checked against its `sha256` when the ERP gives it, then kept in `pont/recus/<id>/` until it becomes a job.
3. **Creation**: the job takes the uuid of the send, the saved setting designated by `prereglage_uuid` and its job ticket: order = `ref_erp`, quantity = `copies`, deadline. A server cut off between creation and cleanup does not create a second job.
4. **Acknowledgement**: the job's first `travail.etat` carries `envoi_id` and the reference.
5. **Linked jobs**: `groupe` (text of 200 characters at most) and `partie` (rank from 1 to 10,000) go together; one without the other is an `envoi_invalide`. The outputs of a part are placed in the queue just after those of parts of lower or equal rank still in the queue, otherwise just before those of a higher rank: the group goes through as a block and in order, even if the renders finish out of order. A priority is still possible.

## Good to know

- **Refusals**: the RIP answers `travail.refuse` with `{ "envoi_id", "ref_erp", "code", "detail" }`. Codes: `envoi_invalide` (malformed item, `detail` names the field), `telechargement_impossible` (404), `empreinte_fausse`, `reglage_inconnu`, and the refusals to create a job (`format_inconnu`, `pdf_illisible`, `polices_manquantes`…).
- **Duplicates**: a send that has been taken or refused is never taken again, even if proposed again; it is forgotten 30 days after the ERP stopped proposing it. An outage (network, 5xx, disk) is not a refusal: the send is taken again on the next cycle.
- With `EKO_RIP_PURGER_ERP_FAITS=1`, a job that came from the ERP is deleted as soon as it is done, without waiting for the retention period: see [Change the server's advanced settings](../rip-administration/rip-modifier-les-reglages-avances-du-serveur.md).
- To act on the queue afterwards, see [Drive the queue from the ERP](rip-piloter-la-file-depuis-l-erp.md).

## See also

- [Understand ERP bridge exchanges](rip-comprendre-les-echanges-du-pont-erp.md)
- [Drive the queue from the ERP](rip-piloter-la-file-depuis-l-erp.md)
- [Drop a file into a watched folder](../rip-travaux/rip-deposer-un-fichier-dans-un-dossier-surveille.md)
