---
titre: View the events sent to the ERP
description: The content of the events from the RIP to the ERP: catalogue, job and output statuses, real consumption and machine status.
sous_rubrique: rip-erp
position: 28
mots_cles: events, catalogue.modifie, travail.etat, sortie.etat, travail.consommation, machine.etat, consumption, ink
slug: rip-view-events-sent-to-erp
traduction_de: rip-consulter-les-evenements-envoyes-a-l-erp
---

The RIP sends the ERP five types of production events. This article describes their content field by field, for the person who reads or integrates the contract.

## Before you start

Each event is wrapped and signed as described in [Verify the signature of ERP bridge events](rip-verifier-la-signature-des-evenements-du-pont-erp.md). The examples below show the `donnees` field.

## Steps

1. **`catalogue.modifie`**: the full catalogue, at bridge start, at every change (machines, cut layers, media, modes, saved settings, ICC profiles) and when the ERP asks for it. Keys: `machines` (printers), `decoupeuses` (cutters), `couches_decoupe` (cut layers), `supports` (media), `modes`, `prereglages` (saved settings), `profils` (profiles), and `serveur` (version). Each object carries its `uuid` (except profiles, identified by `fichier`) and its `nom` (name); `machine_uuid`, `support_uuid`, `mode_uuid` and `decoupeuse_uuid` link them. An unknown field is omitted, never `null`. No prices.
2. **`travail.etat`**: at every change of a job's status.

   ```json
   { "travail_uuid": "…", "ref_erp": null, "etat": "termine", "code_erreur": null, "pages": 2 }
   ```

   A job born from a send by the ERP also carries `envoi_id` and its reference (`ref_erp`); a job in a group also carries `groupe` and `partie`.
3. **`sortie.etat`**: at every change of an output's status.

   ```json
   { "sortie_uuid": "…", "travail_uuid": "…", "ref_erp": null, "page": 1, "machine_uuid": "…",
     "machine": "epson", "operation": "impression", "etat": "en_attente", "code_erreur": null,
     "raison_attente": "attente_support", "planche_uuid": null }
   ```
4. **`travail.consommation`**: at the end of a print, then of a cut (output `terminee`).

   ```json
   { "travail_uuid": "…", "sortie_uuid": "…", "ref_erp": null, "page": 1, "operation": "impression",
     "machine_uuid": "…", "machine": "epson", "support_uuid": "…", "mode_uuid": "…", "couche": null,
     "copies": 1, "surface_m2": 1.2, "longueur_prevue_mm": 1000, "longueur_mm": 1204,
     "longueur_source": "reel", "gache_mm": 204, "encre_ml": { "C": 3.5 }, "encre_sous_seuil_ml": null,
     "encre_ml_estimee": { "C": 3.1 }, "encre_source": "reel", "duree_s": 600, "debut": 1793000000, "fin": 1793000600 }
   ```
5. **`machine.etat`**: the status read from a printer (SNMP, every minute), published only when it changes.

   ```json
   { "machine_uuid": "…", "machine": "epson", "encres": [{ "encre": "BK", "pourcent": 80 }],
     "longueur_restante_m": 37.5, "support_uuid": "…", "laize_mm": 1620,
     "chauffages": [{ "ecran_c": 40, "autre_c": 45 }], "alertes": ["printer cover open"],
     "bac_encre_usagee_pourcent": null, "releve_le": 1793000000 }
   ```

## Good to know

- **Nothing is invented**: without a machine reading (SNMP), `encre_ml`, `longueur_mm` and `gache_mm` are `null`. `encre_source` says where the ink figure comes from: `reel` (measured), `estime` (from the dots placed by the direct driver), `null` (none).
- `encre_ml` is the real ink apportioned at the time of the end: how a drop is split between several prints can be refined afterwards in the RIP's reports.
- `ref_erp` is the project reference of a job sent by the ERP, copied into its outputs and consumption records; `null` for a job created at the RIP.
- In `machine.etat`: `encres` gives the percentage remaining as the printer reports it; `longueur_restante_m` is the length entered when the roll was loaded, counted down by the machine (`null` if unknown); `support_uuid` and `laize_mm` describe the loaded media (entered by the operator or read from the machine); `chauffages` gives two values per heater, `ecran_c` being the one on the printer's screen; `alertes` repeats the machine's texts as they are; `bac_encre_usagee_pourcent` is `null` as long as it is not read on the Epson. The ERP keeps the last `machine.etat` received.
- The media inventory is the `supports` of `catalogue.modifie` and, machine by machine, the loaded media and its remaining length from `machine.etat`. A machine's journal is its `sortie.etat`, its `travail.consommation` and its `machine.etat`, all with `machine_uuid`.

## See also

- [Understand job and output statuses](rip-comprendre-les-etats-des-travaux-et-des-sorties.md)
- [Understand ERP bridge identifiers](rip-comprendre-les-identifiants-du-pont-erp.md)
- [Understand ERP bridge exchanges](rip-comprendre-les-echanges-du-pont-erp.md)
