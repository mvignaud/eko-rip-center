---
titre: Estimate a project from the ERP
description: The ERP asks the RIP for a project's area, ink, duration and roll length, already received or to be quoted before sending, with no amounts.
sous_rubrique: rip-erp
position: 32
mots_cles: estimate, quote, travail.estimation, area, ink, length, duration, roll width, tiling
slug: rip-estimate-project-from-erp
traduction_de: rip-estimer-un-dossier-depuis-l-erp
---

Before producing, the ERP can ask what a project will consume. The RIP answers with physical quantities, never a price. This article describes the request and the response.

## Before you start

The bridge is turned on. For a project already received, its reference is enough; for a quote, the ERP sends the file to estimate.

## Steps

1. Send in `estimations` an item `{ "id", "ref_erp" }` for a project already received, or the shape of an item of `travaux` (`prereglage_uuid`, `copies`, `fichier`) for a file to estimate before sending.
2. For a file to estimate: the RIP downloads it like a send (signed, SHA-256 verified), renders it with the designated setting in a hidden job (not in the list nor in the queue, no `travail.etat`, all its pages together), estimates it then deletes it.
3. Read the `travail.estimation` response:

   ```json
   { "estimation_id": "…", "ref_erp": "…", "code": null, "travaux": 1, "pages": 2, "copies": 3,
     "surface_m2": 4.8, "encres": [{ "nom": "C", "ml": 12.5 }], "ml": 40.1,
     "duree_impression_min": 25, "machine_uuid": "…", "laize_mm": 1620, "longueur_mm": 3100, "les": 0 }
   ```
4. Interpret the figures: those from each page's sheet, multiplied by the job's copies (its quantity, at least one); area, ink per plate (cut spot colours excluded), duration on the first printer in the catalogue; roll length of all the copies nested together on the loaded roll width of that printer, with its nesting setting and its margins, with no target length.

## Good to know

- `longueur_mm` is `null` if a page does not fit on the roll width; `duree_impression_min` is empty with no known throughput; `les` counts the tiling panels, copies included.
- **No amounts**: neither ink cost nor media cost. The ERP does its own pricing.
- **Scope**: the rendered jobs born from an ERP send that carry this `ref_erp`. A failed job is left aside; a job still rendering makes the request wait for the next round (30 seconds at most). Excluded pages do not count.
- **Codes**: `null`, `aucun_travail` (no rendered job for this project, no figures), `estimation_invalide` (malformed item, `detail` names the field). A file that cannot be downloaded or rendered is answered with `travail.estimation` carrying the refusal code (`empreinte_fausse`, `reglage_inconnu`, `pdf_illisible`, `rendu_echoue`…) and its `detail`, never with `travail.refuse`.
- **Duplicates**: like actions, a request that has been taken is never taken again.
- Fill in the machines' **Throughput** (*Cadence*) so that the duration is estimated.

## See also

- [Receive the jobs sent by the ERP](rip-recevoir-les-travaux-envoyes-par-l-erp.md)
- [Drive the queue from the ERP](rip-piloter-la-file-depuis-l-erp.md)
- [Fix an estimated duration that does not match](../rip-depannage/rip-duree-estimee-incorrecte.md)
