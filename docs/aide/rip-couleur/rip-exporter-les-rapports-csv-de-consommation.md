---
titre: Exporter les rapports CSV de consommation
description: Téléchargez les rapports par impression, par jour et d'estimations, avec leurs colonnes, leur format et la limitation à une période.
sous_rubrique: rip-couleur
position: 50
mots_cles: csv, rapport, consommation, estimations, par jour, par impression, période, tableur
---

Les rapports CSV alimentent un tableur pour suivre l'encre et le support consommés. Cet article s'adresse à l'administrateur.

## Avant de commencer

Il vous faut un compte administrateur. Le journal contient des impressions (voir [Consulter le journal des impressions](rip-consulter-le-journal-des-impressions.md)).

## Étapes

1. Pour le **réel** : dans **Système**, section **Journal des impressions**, cliquez **Rapport CSV par impression** (une ligne par impression) ou **Rapport CSV par jour** (une ligne par jour et par machine).
2. Pour l'**estimé** : dans la section **Coûts d'encre et de support**, cliquez **Exporter les estimations (CSV)** : une ligne par page de chaque travail rendu, imprimé ou non. **Exporter la consommation (CSV)** donne le réel.
3. Pour limiter à une période, ajoutez `?depuis=&jusqua=` (secondes Unix) à l'adresse : `/api/rapports/consommation.csv` pour le réel, `/api/rapports/estimations.csv` pour l'estimé (date de création du travail).
4. Ouvrez le fichier dans votre tableur (séparateur `;`).

## Bon à savoir

- Conventions communes : séparateur `;`, décimales à la virgule, colonnes toujours les mêmes et dans le même ordre, dernière ligne `total` égale à la somme des lignes.
- Rapport par impression : identité (date UTC, machine, travail, page, nombre d'impressions, support, mode, dpi, opérateur), surface, longueurs prévue et réelle, gâche, méthode d'estimation (`points` ou `non calculable`), encre estimée et réelle côte à côte (totaux, `sous_seuil`, écart en %, puis une paire `<encre>_estime_ml` / `<encre>_reel_ml` pour chacune des 11 encres du catalogue), coûts et base HT/TTC.
- Rapport d'estimations : date UTC, travail, page, support, surface, surface de support, ml estimés par encre (`BK_ml`, `C_ml`, `M_ml`, `Y_ml`, `WH_ml`, `MS_ml`, tons directs dans `autres_ml`), coûts encre et support, total et base.

## Voir aussi

- [Consulter le journal des impressions](rip-consulter-le-journal-des-impressions.md)
- [Régler les coûts d'encre et de support](rip-regler-les-couts-d-encre-et-de-support.md)
- [Corriger l'estimation d'encre avec les facteurs de correction](rip-corriger-l-estimation-d-encre.md)
