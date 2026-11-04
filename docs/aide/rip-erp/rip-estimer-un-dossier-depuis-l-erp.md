---
titre: Estimer un dossier depuis l'ERP
description: L'ERP demande au RIP la surface, l'encre, la durée et la longueur de rouleau d'un dossier, déjà reçu ou à chiffrer avant l'envoi, sans aucun montant.
sous_rubrique: rip-erp
position: 32
mots_cles: estimation, devis, travail.estimation, surface, encre, longueur, durée, laize, mosaïque
---

Avant de produire, l'ERP peut demander ce qu'un dossier consommera. Le RIP répond avec des quantités physiques, jamais un prix. Cet article décrit la demande et la réponse.

## Avant de commencer

Le pont est activé. Pour un dossier déjà reçu, il suffit de sa référence ; pour un devis, l'ERP envoie le fichier à estimer.

## Étapes

1. Envoyez dans `estimations` un élément `{ "id", "ref_erp" }` pour un dossier déjà reçu, ou la forme d'un élément de `travaux` (`prereglage_uuid`, `copies`, `fichier`) pour un fichier à estimer avant l'envoi.
2. Pour un fichier à estimer : le RIP le télécharge comme un envoi (signé, SHA-256 vérifié), le rend avec le réglage désigné dans un travail caché (ni dans la liste ni dans la file, sans `travail.etat`, toutes ses pages ensemble), l'estime puis l'efface.
3. Lisez la réponse `travail.estimation` :

   ```json
   { "estimation_id": "…", "ref_erp": "…", "code": null, "travaux": 1, "pages": 2, "copies": 3,
     "surface_m2": 4.8, "encres": [{ "nom": "C", "ml": 12.5 }], "ml": 40.1,
     "duree_impression_min": 25, "machine_uuid": "…", "laize_mm": 1620, "longueur_mm": 3100, "les": 0 }
   ```
4. Interprétez les chiffres : ceux de la fiche de chaque page, multipliés par les copies du travail (sa quantité, une au moins) ; surface, encre par plaque (tons de découpe exclus), durée sur la première imprimante du catalogue ; longueur de rouleau de toutes les copies imbriquées ensemble sur la laize chargée de cette imprimante, avec son réglage d'imbrication et ses marges, sans longueur cible.

## Bon à savoir

- `longueur_mm` vaut `null` si une page ne tient pas sur la laize ; `duree_impression_min` est vide sans cadence connue ; `les` compte les lés de mosaïque, copies comprises.
- **Aucun montant** : ni coût d'encre ni de support. L'ERP fait ses prix.
- **Portée** : les travaux rendus nés d'un envoi de l'ERP qui portent ce `ref_erp`. Un travail en échec est laissé de côté ; un travail encore en rendu fait attendre la demande au tour suivant (30 secondes au plus). Les pages exclues ne comptent pas.
- **Codes** : `null`, `aucun_travail` (aucun travail rendu pour ce dossier, sans chiffres), `estimation_invalide` (élément mal formé, `detail` nomme le champ). Un fichier qui ne se télécharge pas ou ne se rend pas répond par `travail.estimation` avec le code du refus (`empreinte_fausse`, `reglage_inconnu`, `pdf_illisible`, `rendu_echoue`…) et son `detail`, jamais par `travail.refuse`.
- **Doublons** : comme les actions, une demande prise n'est jamais reprise.
- Renseignez la **Cadence** des machines pour que la durée soit estimée.

## Voir aussi

- [Recevoir les travaux envoyés par l'ERP](rip-recevoir-les-travaux-envoyes-par-l-erp.md)
- [Piloter la file depuis l'ERP](rip-piloter-la-file-depuis-l-erp.md)
- [Corriger une durée estimée qui ne correspond pas](../rip-depannage/rip-duree-estimee-incorrecte.md)
