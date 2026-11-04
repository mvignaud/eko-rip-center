---
titre: Correct the ink estimate with correction factors
description: Apply the factor proposed by the server (actual ink ÷ estimated) per machine, resolution and ink for more accurate estimates.
sous_rubrique: rip-couleur
position: 51
mots_cles: correction factor, estimated ink, actual ink, apply, reset to 1, log
slug: rip-correct-the-ink-estimate-with-correction-factors
traduction_de: rip-corriger-l-estimation-d-encre
---

The estimated ink can differ from the actual ink. A correction factor per machine, resolution and ink brings the two closer. This article is for the administrator.

## Before you start

You need measured prints: through the direct driver, with SNMP reading (see [Consult the print log](rip-consulter-le-journal-des-impressions.md)). Without them, the screen shows "Rien à proposer" (nothing to suggest).

## Steps

1. Open **Administration…** (*Administration…*), **System** (*Système*) view, **Print log** (*Journal des impressions*) section, **Correction factors for the estimated ink** (*Facteurs de correction de l'encre estimée*) block.
2. Read the proposed factor: *actual ink ÷ estimated ink* over the last 10 measured prints of each machine, resolution and ink combination.
3. Click **Apply** (*Appliquer*) to validate it.
4. To remove it, click **Reset to 1** (*Remettre à 1*).

## Good to know

- The factor is 1 by default.
- It applies only to the following prints: the log keeps each print's factor.
- Retention and factors are included in the configuration backup.

## See also

- [Consult the print log](rip-consulter-le-journal-des-impressions.md)
- [Set the ink and media costs](rip-regler-les-couts-d-encre-et-de-support.md)
- [Back up the server configuration](../rip-administration/rip-sauvegarder-la-configuration.md)
