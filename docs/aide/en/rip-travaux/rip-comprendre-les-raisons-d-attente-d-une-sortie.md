---
titre: Understand why an output waits
description: Read the red badge on an output that will not go (machine unreachable, other media, too wide, print not finished...) and act on it.
sous_rubrique: rip-travaux
position: 44
mots_cles: waiting reason, red badge, unreachable, other media, too wide, print not finished, nesting, excluded page
slug: rip-understand-why-an-output-waits
traduction_de: rip-comprendre-les-raisons-d-attente-d-une-sortie
---

An output that cannot go says so with a red badge, and lets the following ones through. This article is the reference for each reason and what to do.

## Before you start

An output is in the queue (see [Manage a machine's queue](rip-gerer-la-file-d-une-machine.md)). Hover over the badge to read the detail. An output held by an operator simply stays **Held** (*Suspendue*): that is not a waiting reason.

## Steps

1. Find the red badge on the output's row in **To print** (*À imprimer*).
2. Look up its reason in the table below.
3. Apply the action given.
4. If needed, click **Redo** (*Refaire*) or **Resume** (*Reprendre*) once the cause is gone.

| Reason | What it means | What to do |
|---|---|---|
| **Machine unreachable** (*Machine injoignable*) | The machine is unplugged or missing from the catalogue. | Nothing: the output goes as soon as the machine answers. Check the cable, the address and the machine's record. |
| **Missing file** (*Fichier manquant*) | The render or the path cannot be found. | Rerun the job (**Rerun**, *Relancer*). |
| **Too wide** (*Trop large*) | The page, marks included, exceeds the width of the machine or of the loaded roll. | Shrink the page, or load a wider roll. |
| **Other media** (*Autre support*) | The loaded media is not the job's media. | Load the right media: the output goes by itself. Or choose the loaded media by hand. |
| **Print not finished** (*Impression pas finie*) | A cut waits until the print of its page is finished. | Wait for the print to finish. |
| **Cutter unavailable** (*Découpeuse indisponible*) | A print waits until the assigned cutter is usable, so as not to waste ink. | Restore the cutter (see [Choose the cutter for a job](rip-choisir-la-decoupeuse-d-un-travail.md)). |
| **Nesting** (*En imbrication*) | The print waits for its sheet. | See [Follow a sheet in the queue](rip-suivre-une-planche-dans-la-file.md). |
| **Page excluded** (*Page exclue*) | The **Print this page** (*Imprimer cette page*) box is unticked. | Tick it again. |
| **Waiting for the proof** (*Attend le BAT*) | The original waits for the proof on another machine. | See [Approve a proof](rip-valider-un-bat.md). |

## Good to know

- A print placed on a sheet becomes **Nested** (*Imbriquée*; "on" the sheet's name): it no longer goes by itself, the sheet prints it. It cannot be **Redone** until its sheet is printed (it would be printed twice); **Remove** (*Retirer*) is still possible.
- An output interrupted by a server stop does not restart by itself (a cut cannot resume in the middle): it goes to *Échec* (failed), and **Redo** restarts it.
- Unknown loaded media: nothing is blocked. A media number unknown to the catalogue is flagged.
- A page that is unticked (**Print this page**) cannot be queued by hand either.

## See also

- [Manage a machine's queue](rip-gerer-la-file-d-une-machine.md)
- [The output to the machine failed](../rip-depannage/rip-sortie-vers-la-machine-echouee.md)
- [Read the media loaded in the Epson](../rip-couleur/rip-lire-le-media-charge-dans-l-epson.md)
