---
titre: Duplicate a cutter for another set of marks
description: Create a variant of a cutter with its own set of marks, for the same physical machine.
sous_rubrique: rip-machines
position: 48
mots_cles: variant, duplicate, set of marks, cutter, kona, registration
slug: rip-duplicate-cutter-for-another-set-of-marks
traduction_de: rip-dupliquer-une-decoupeuse-pour-un-autre-jeu-de-reperes
---

A cutter variant lets you use a different registration with the same physical machine. This article is for the administrator.

## Before you start

A cutter is declared in the catalogue (see [Add a machine to the RIP](rip-ajouter-une-machine.md)).

## Steps

1. Open **Administration…**, the **System** view (*Système*), the **Machines** section, the cutter's page.
2. Click **Duplicate (other set of marks)** (*Dupliquer (autre jeu de repères)*).
3. In the variant, choose the **Set of marks** (*Jeu de repères*), or **— celui du travail —** ("the job's own").
4. Click **Save** (*Enregistrer*).
5. When you send a job, assign it the variant in **Cutter** (*Découpeuse*).

## Good to know

- Connection, model and cut settings are taken from the original machine at every save.
- A job assigned to the variant takes its marks; its cut goes through the original machine, one output at a time.

## See also

- [Choose the Kona registration](rip-choisir-le-reperage-kona.md)
- [Choose a job's cutter](../rip-travaux/rip-choisir-la-decoupeuse-d-un-travail.md)
- [Understand a machine's fields](rip-comprendre-les-champs-d-une-machine.md)
