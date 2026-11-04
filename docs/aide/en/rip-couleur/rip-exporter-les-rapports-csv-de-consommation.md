---
titre: Export the CSV consumption reports
description: Download the per-print, per-day and estimate reports, with their columns, their format and how to limit them to a period.
sous_rubrique: rip-couleur
position: 50
mots_cles: csv, report, consumption, estimates, per day, per print, period, spreadsheet
slug: rip-export-the-csv-consumption-reports
traduction_de: rip-exporter-les-rapports-csv-de-consommation
---

The CSV reports feed a spreadsheet to track the ink and media consumed. This article is for the administrator.

## Before you start

You need an administrator account. The log contains prints (see [Consult the print log](rip-consulter-le-journal-des-impressions.md)).

## Steps

1. For the **actual** figures: in **System** (*Système*), **Print log** (*Journal des impressions*) section, click **CSV report per print** (*Rapport CSV par impression*; one line per print) or **CSV report per day** (*Rapport CSV par jour*; one line per day and per machine).
2. For the **estimated** figures: in the **Ink and media costs** (*Coûts d'encre et de support*) section, click **Export the estimates (CSV)** (*Exporter les estimations (CSV)*): one line per page of each rendered job, printed or not. **Export the consumption (CSV)** (*Exporter la consommation (CSV)*) gives the actual figures.
3. To limit to a period, add `?depuis=&jusqua=` (Unix seconds) to the address: `/api/rapports/consommation.csv` for the actual figures, `/api/rapports/estimations.csv` for the estimated ones (job creation date).
4. Open the file in your spreadsheet (separator `;`).

## Good to know

- Common conventions: separator `;`, decimal comma, columns always the same and in the same order, last line `total` equal to the sum of the lines.
- Per-print report: identity (UTC date, machine, job, page, number of prints, media, mode, dpi, operator), area, planned and actual lengths, waste, estimation method (`points` or `non calculable`), estimated and actual ink side by side (totals, `sous_seuil`, gap in %, then a pair `<ink>_estime_ml` / `<ink>_reel_ml` for each of the 11 inks of the catalogue), costs and the excl.-tax / incl.-tax basis.
- Estimates report: UTC date, job, page, media, area, media area, estimated ml per ink (`BK_ml`, `C_ml`, `M_ml`, `Y_ml`, `WH_ml`, `MS_ml`, spot colours in `autres_ml`), ink and media costs, total and basis.

## See also

- [Consult the print log](rip-consulter-le-journal-des-impressions.md)
- [Set the ink and media costs](rip-regler-les-couts-d-encre-et-de-support.md)
- [Correct the ink estimate with correction factors](rip-corriger-l-estimation-d-encre.md)
