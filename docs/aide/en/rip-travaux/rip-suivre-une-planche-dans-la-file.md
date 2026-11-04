---
titre: Follow a sheet being prepared in the queue
description: Read the sheet being prepared, what nests together, the length and fill rate, and start it with Print now.
sous_rubrique: rip-travaux
position: 68
mots_cles: nesting, sheet being prepared, print now, abandoned sheet, trigger, fill rate
slug: rip-follow-a-sheet-in-the-queue
traduction_de: rip-suivre-une-planche-dans-la-file
---

When a printer nests its prints, the sheet is prepared in the queue before it goes out. This article explains how to read it and start it.

## Before you start

Nesting is set on the printer (see [Set up a printer's nesting](../rip-machines/rip-regler-l-imbrication-d-une-imprimante.md)).

## Steps

1. Open the **Queue** (*File d'attente*) view (or **Production**): the printer's column shows "Imbrication : *strategy*" (nesting: strategy) and, above its outputs, each **Sheet being prepared — n pages** (*Planche en préparation — n pages*).
2. Read the sheet: the media, the cutter ("none") and the mirror, and **NestTrimBox outline** if requested; the imposition drawing; **Length … mm · fill rate … %**.
3. Read the list of pages: name, page, copies, order and due date. Untick **Print this page** (*Imprimer cette page*) to take a page out of the sheet and the queue, until it is ticked again.
4. Click **Sheet PDF** (*PDF de la planche*) to see the exact PDF that will be printed.
5. Wait for the trigger, or click **Print now** (*Imprimer maintenant*): the sheet goes without waiting.

## Good to know

- *Déclencheur atteint : la planche part.* (trigger reached: the sheet goes) is shown when that is the case.
- When the sheet goes, it becomes a job "Imbrication *machine* — n page(s)" (nesting machine — n page(s)), rendered then queued on the printer (and on the group's cutter if it has one). Its ticket takes up the orders placed, the nearest due date and, in notes, each page with its copies. The prints placed become **Nested** (*Imbriquée*).
- **What nests together**: only jobs with the same **media**, the same **print mode**, the same **cutter** ("none" included: a job to cut on the Kona and a job with no cut make two sheets) and the same **mirror** share a sheet; and what the sheet has only once: the **resolution**, the **registration**, the **colour** (profiles, intent), the **white**, the ink layers, the **Background under the marks**, the **Paths to cut** and the **Cutting tolerance**; finally the **cut settings chosen per layer**. A layer unticked on a job is not cut on the sheet.
- Only PDFs can be nested; a page wider than the roll fits on no sheet.
- **Abandoned sheet**: render failed, sheet job deleted, or the sheet's print cancelled, failed or removed before it was printed. Its jobs return to the queue by themselves, in their previous state (a held cut returns held), and to the sheet being prepared. Nothing returns from a printed sheet.
- An abandoned sheet can no longer be redone: **Redo** on its output, **Rerun** on its job or queueing it again is refused (*Planche abandonnée : ses travaux sont revenus en file…*, abandoned sheet: its jobs have returned to the queue...), otherwise its jobs would print twice. **Print now** on the sheet being prepared makes a correct one. A printed sheet, on the other hand, can be **Redone** (deliberate reprint).

## See also

- [Set up a printer's nesting](../rip-machines/rip-regler-l-imbrication-d-une-imprimante.md)
- [Set the triggers that start a sheet](../rip-machines/rip-regler-les-declencheurs-de-depart-d-une-planche.md)
- [Manage a machine's queue](rip-gerer-la-file-d-une-machine.md)
