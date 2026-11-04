---
titre: Fix an estimated duration that does not match
description: Measure the machine's real throughput and enter it so that the estimated duration of jobs is accurate.
sous_rubrique: rip-depannage
position: 24
mots_cles: estimated duration, throughput, m2 per hour, roll width, estimate, machine
slug: rip-fix-incorrect-estimated-duration
traduction_de: rip-duree-estimee-incorrecte
---

The duration announced for a job comes from the machine's **Throughput** (*Cadence*). If it is wrong, so is the duration.

## Before you start

You are an administrator. Allow for a test job printed on your usual roll width.

## Steps

1. Print a test job and time it.
2. Calculate the real throughput: square metres printed per hour, on the roll width used.
3. Open **Administration…**, the **System** view (*Système*), the **Machines** section, and the machine's page.
4. Enter the value in the **Throughput** field.
5. Save, then compare the announced duration of the next job with its real duration.

## Good to know

- With no known throughput, no duration is given: the estimate sent to the ERP leaves it empty.
- Throughput depends on the print mode: measure it with the most used mode.

## See also

- [Add a machine](../rip-machines/rip-ajouter-une-machine.md)
- [Estimate a project from the ERP](../rip-erp/rip-estimer-un-dossier-depuis-l-erp.md)
- [Manage a machine's queue](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
