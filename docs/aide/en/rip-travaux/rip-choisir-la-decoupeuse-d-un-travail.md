---
titre: Choose the cutter for a job
description: Assign a cutter to a job and understand why printing waits until the cutter is usable.
sous_rubrique: rip-travaux
position: 62
mots_cles: cutter, cut after print, cutter unavailable, paths to cut, cut only
slug: rip-choose-the-cutter-for-a-job
traduction_de: rip-choisir-la-decoupeuse-d-un-travail
---

The cutter chosen when you send the job receives the automatic cut of each page. This article explains how it affects the queue.

## Before you start

A cutter is declared in the catalogue (see [Add a machine to the RIP](../rip-machines/rip-ajouter-une-machine.md)).

## Steps

1. Click **Import** (*Importer*).
2. Choose the **Mode**: **Print (and cut)** (*Imprimer (et découper)*) or **Cut only** (*Découper seulement*; nothing is printed).
3. In **Paths to cut** (*Tracés à découper*), choose the recognised cut spot colours or **All paths** (*Tous les tracés*).
4. In **Cutter** (*Découpeuse*), choose the machine (**— none —**, *— aucune —*, if the job is not cut).
5. Click **Send to the RIP** (*Envoyer au RIP*).
6. For a page that is already rendered, click **Cut on…** (*Découper sur…*) in the inspector.

## Good to know

- As long as the cutter is not usable (missing, offline, badly connected, or a layer with no setting on it), printing waits: nothing goes out, so as not to waste ink or media (reason **Cutter unavailable**, *Découpeuse indisponible*).
- A cut waits until the print of its page is 100% finished (reason **Print not finished**, *Impression pas finie*).
- Jobs only share a sheet if they have the same cutter ("none" included).
- With **Duplicate (other set of marks)** (*Dupliquer (autre jeu de repères)*), a variant of the cutter takes its own set of marks: see [Duplicate a cutter for another set of marks](../rip-machines/rip-dupliquer-une-decoupeuse-pour-un-autre-jeu-de-reperes.md).

## See also

- [Understand why an output waits](rip-comprendre-les-raisons-d-attente-d-une-sortie.md)
- [Queue pages automatically](rip-mettre-en-file-automatiquement.md)
- [Order the cut layers](rip-ordonner-les-couches-de-decoupe.md)
