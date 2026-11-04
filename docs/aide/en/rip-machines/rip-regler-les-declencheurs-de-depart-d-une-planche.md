---
titre: Set a sheet's start triggers
description: Decide when a nested sheet goes out: delay, no new arrivals, area, target length, percentage or full row.
sous_rubrique: rip-machines
position: 44
mots_cles: triggers, start after, target length, area, full row, print now, next sheet
slug: rip-set-sheet-start-triggers
traduction_de: rip-regler-les-declencheurs-de-depart-d-une-planche
---

A nested sheet goes out when its trigger is reached. This article describes each trigger, for the administrator.

## Before you start

Nesting is turned on for the printer (see [Set up a printer's nesting](rip-regler-l-imbrication-d-une-imprimante.md)).

## Steps

1. Open the printer's page, **Machines** section of the **System** view (*Système*).
2. Fill in the triggers you want. An empty field = no trigger; with several, the first one reached fires.

   | Trigger | Effect |
   |---|---|
   | **Start after (min)** (*Départ après (min)*) | Minutes since the **first** job of the sheet arrived. |
   | **Start with no arrival for (min)** (*Départ sans arrivée depuis (min)*) | Minutes with no **new** job (since the last one arrived). |
   | **Start at (m²)** (*Départ à (m²)*) | Area of the placed jobs, copies included. |
   | **Target length (mm)** (*Longueur cible (mm)*) | Jobs that would take it past the limit wait for the next sheet (badge *attend la planche suivante*, "waiting for the next sheet"), and a full sheet goes out by itself. A single job longer than the target still goes out, alone. |
   | **Start at (% of target)** (*Départ à (% de la cible)*) | The sheet goes out when it reaches this percentage of the target length (requires a target length). |
   | **Start at length (mm)** (*Départ à la longueur (mm)*) | The sheet goes out as soon as it reaches this length; nothing is postponed (the job that crosses it goes out with it). |
   | **Full row goes out alone** (*Rangée pleine sortie seule*) | The job that would open a second row waits, and the full row goes out alone. |

3. Click **Save** (*Enregistrer*).

## Good to know

- With no trigger at all, the sheet waits until you click **Print now** (*Imprimer maintenant*).
- **Print now** is in the sheet being prepared in the **Queue** view (*File d'attente*).

## See also

- [Set up a printer's nesting](rip-regler-l-imbrication-d-une-imprimante.md)
- [Follow a sheet being prepared in the queue](../rip-travaux/rip-suivre-une-planche-dans-la-file.md)
- [Create a sheet to group prints](../rip-travaux/rip-creer-une-planche.md)
