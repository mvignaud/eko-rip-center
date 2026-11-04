---
titre: Understand job and output statuses
description: The statuses of a job (render) and of an output (print or cut) published for the ERP, what they mean and what happens after an outage.
sous_rubrique: rip-erp
position: 27
mots_cles: statuses, job, output, en_attente, en_cours, termine, echec, suspendue, annulee, imbriquee, interrompu
slug: rip-understand-job-and-output-statuses
traduction_de: rip-comprendre-les-etats-des-travaux-et-des-sorties
---

The RIP publishes the status of each job and each output for the ERP. This article describes these statuses and their suggested mapping on the ERP side. They can also be read programmatically in `GET /api/erp/contrat` (`etats`).

## Before you start

A **job** is the render of a file; an **output** is the sending of a page to a machine (print or cut). See the [glossary](../rip-premiers-pas/rip-consulter-le-glossaire.md).

## Steps

1. Read the statuses of a **job**:

   | Status | Meaning |
   |---|---|
   | `en_attente` | received, waiting for the render |
   | `en_cours` | render in progress |
   | `termine` | render finished: ready to go out |
   | `echec` | render impossible (stable error code) |
2. Read the statuses of an **output**:

   | Status | Meaning |
   |---|---|
   | `en_attente` | queued for a machine |
   | `suspendue` | held by an operator |
   | `en_cours` | being sent to the machine |
   | `terminee` | sent to the machine (print or cut) |
   | `echec` | send impossible (stable error code) |
   | `annulee` | withdrawn before sending |
3. For an output placed on a nesting sheet, note the `imbriquee` status: the sheet, a separate job, prints it, and the output becomes `terminee` when the sheet is printed. Consumption is then carried by the sheet.
4. Apply the suggested mapping on the ERP side: job `termine`: "ready to print"; `terminee` output of a printer: "printing"; `terminee` output of a cutter: "cut"; `echec`: "blocked", with the code.

## Good to know

- After an outage, a job that was `en_cours` becomes `echec` with the code `interrompu` ("interrupted"): it must be restarted, it does not restart by itself. An output that was `en_cours` also becomes `echec` with `interrompu`: a cut cannot be resumed halfway.
- For now, `terminee` means "handed to the machine", not "out of the machine".
- A newly documented status is a compatible addition: the ERP must ignore those it does not know.
- Status changes are published by the events described in [View the events sent to the ERP](rip-consulter-les-evenements-envoyes-a-l-erp.md).

## See also

- [View the events sent to the ERP](rip-consulter-les-evenements-envoyes-a-l-erp.md)
- [Recover after a power cut](../rip-administration/rip-reprendre-apres-une-coupure-de-courant.md)
- [Manage a machine's queue](../rip-travaux/rip-gerer-la-file-d-une-machine.md)
