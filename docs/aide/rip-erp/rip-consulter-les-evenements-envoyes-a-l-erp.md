---
titre: Consulter les événements envoyés à l'ERP
description: Le contenu des événements du RIP vers l'ERP : catalogue, état des travaux et des sorties, consommation réelle et état des machines.
sous_rubrique: rip-erp
position: 28
mots_cles: événements, catalogue.modifie, travail.etat, sortie.etat, travail.consommation, machine.etat, consommation, encre
---

Le RIP envoie à l'ERP cinq types d'événements de production. Cet article décrit leur contenu champ par champ, pour la personne qui lit ou intègre le contrat.

## Avant de commencer

Chaque événement est enveloppé et signé comme décrit dans [Vérifier la signature des événements du pont ERP](rip-verifier-la-signature-des-evenements-du-pont-erp.md). Les exemples ci-dessous montrent le champ `donnees`.

## Étapes

1. **`catalogue.modifie`** : le catalogue complet, au démarrage du pont, à chaque modification (machines, couches de découpe, supports, modes, réglages enregistrés, profils ICC) et quand l'ERP le demande. Clés : `machines` (imprimantes), `decoupeuses`, `couches_decoupe`, `supports`, `modes`, `prereglages`, `profils`, et `serveur` (version). Chaque objet porte son `uuid` (sauf les profils, identifiés par `fichier`) et son `nom` ; `machine_uuid`, `support_uuid`, `mode_uuid` et `decoupeuse_uuid` les relient. Un champ inconnu est omis, jamais `null`. Aucun prix.
2. **`travail.etat`** : à chaque changement d'état d'un travail.

   ```json
   { "travail_uuid": "…", "ref_erp": null, "etat": "termine", "code_erreur": null, "pages": 2 }
   ```

   Un travail né d'un envoi de l'ERP porte en plus `envoi_id` et sa référence (`ref_erp`) ; un travail d'un groupe porte aussi `groupe` et `partie`.
3. **`sortie.etat`** : à chaque changement d'état d'une sortie.

   ```json
   { "sortie_uuid": "…", "travail_uuid": "…", "ref_erp": null, "page": 1, "machine_uuid": "…",
     "machine": "epson", "operation": "impression", "etat": "en_attente", "code_erreur": null,
     "raison_attente": "attente_support", "planche_uuid": null }
   ```
4. **`travail.consommation`** : à la fin d'une impression, puis d'une découpe (sortie `terminee`).

   ```json
   { "travail_uuid": "…", "sortie_uuid": "…", "ref_erp": null, "page": 1, "operation": "impression",
     "machine_uuid": "…", "machine": "epson", "support_uuid": "…", "mode_uuid": "…", "couche": null,
     "copies": 1, "surface_m2": 1.2, "longueur_prevue_mm": 1000, "longueur_mm": 1204,
     "longueur_source": "reel", "gache_mm": 204, "encre_ml": { "C": 3.5 }, "encre_sous_seuil_ml": null,
     "encre_ml_estimee": { "C": 3.1 }, "encre_source": "reel", "duree_s": 600, "debut": 1793000000, "fin": 1793000600 }
   ```
5. **`machine.etat`** : l'état relevé d'une imprimante (SNMP, toutes les minutes), publié seulement quand il change.

   ```json
   { "machine_uuid": "…", "machine": "epson", "encres": [{ "encre": "BK", "pourcent": 80 }],
     "longueur_restante_m": 37.5, "support_uuid": "…", "laize_mm": 1620,
     "chauffages": [{ "ecran_c": 40, "autre_c": 45 }], "alertes": ["printer cover open"],
     "bac_encre_usagee_pourcent": null, "releve_le": 1793000000 }
   ```

## Bon à savoir

- **Rien n'est inventé** : sans relevé de la machine (SNMP), `encre_ml`, `longueur_mm` et `gache_mm` valent `null`. `encre_source` dit d'où vient l'encre donnée : `reel` (mesuré), `estime` (par les points posés du pilote direct), `null` (aucune).
- `encre_ml` est l'encre réelle répartie au moment de la fin : la répartition d'une baisse entre plusieurs impressions peut s'affiner ensuite dans les rapports du RIP.
- `ref_erp` est la référence du dossier d'un travail envoyé par l'ERP, recopiée dans ses sorties et ses consommations ; `null` pour un travail créé au RIP.
- Dans `machine.etat` : `encres` donne le pourcentage restant tel que l'imprimante le donne ; `longueur_restante_m` est la longueur saisie au chargement du rouleau, décomptée par la machine (`null` si inconnue) ; `support_uuid` et `laize_mm` décrivent le support chargé (saisi par l'opérateur ou lu sur la machine) ; `chauffages` donne deux valeurs par chauffage, `ecran_c` étant celle de l'écran de l'imprimante ; `alertes` reprend les textes de la machine tels quels ; `bac_encre_usagee_pourcent` vaut `null` tant qu'il n'est pas lu sur l'Epson. L'ERP garde le dernier `machine.etat` reçu.
- L'inventaire des supports, ce sont les `supports` de `catalogue.modifie` et, machine par machine, le support chargé et sa longueur restante de `machine.etat`. Le journal d'une machine, ce sont ses `sortie.etat`, ses `travail.consommation` et ses `machine.etat`, tous avec `machine_uuid`.

## Voir aussi

- [Comprendre les états des travaux et des sorties](rip-comprendre-les-etats-des-travaux-et-des-sorties.md)
- [Comprendre les identifiants du pont ERP](rip-comprendre-les-identifiants-du-pont-erp.md)
- [Comprendre les échanges du pont ERP](rip-comprendre-les-echanges-du-pont-erp.md)
