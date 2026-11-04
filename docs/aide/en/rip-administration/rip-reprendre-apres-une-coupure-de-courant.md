---
titre: Recover after a power cut
description: What happens to work after a power cut: interrupted jobs to retry, status files recovered automatically, restoring if a file is unreadable.
sous_rubrique: rip-administration
position: 23
mots_cles: power cut, interrupted, status files, prec, unreadable, ups, restore, retry
slug: rip-recover-after-a-power-cut
traduction_de: rip-reprendre-apres-une-coupure-de-courant
---

A power cut interrupts the render and the transfers in progress, but the settings files survive. This article says what to check at restart.

## Before you start

The server PC is back on and the service restarted by itself. You have access to the client's **System** (*Système*) view with an administrator account.

## Steps

1. In the client, open **Administration…** (*Administration…*) then the **System** view.
2. Look for the **Files and folders** (*Fichiers et dossiers*) note: it appears only if a status file was found damaged or a folder missing.
3. If it says "<file> illisible (coupure ?) : version précédente reprise" (file unreadable, power cut?: previous version restored), there is nothing to do: the server recovered the previous version (`.prec`) by itself. A missing folder is recreated.
4. If it says "<file> illisible, sans version précédente : à restaurer" (unreadable, with no previous version: to be restored), restore the last configuration backup: see [Rebuild a server from a backup](rip-remonter-un-serveur.md). The damaged file is kept as a `.illisible` copy for analysis.
5. Go back to the **Production** (*Production*) view and use **Retry** (*Relancer*) on the jobs that ended in "interrompu" (interrupted) failure.
6. Redo the outputs that were being sent: they are failed and are restarted from the history with **Redo** (*Refaire*).

## Good to know

- A job whose render was in progress ends in failure with the code `interrompu`; it does not restart by itself.
- An output being sent ends in failure: a cut does not resume in the middle, it must be redone.
- Each status file is written to disk before being put in place, and its previous version is kept.
- Damaged accounts never open access: sign-in remains required.
- To limit the damage, plug the server, the switch and, if possible, the internet box into a UPS that lasts a few minutes and tells the server to shut down cleanly (for example with `nut`).

## See also

- [Choose the server computer](../rip-premiers-pas/rip-choisir-le-poste-serveur.md)
- [Back up the server configuration](rip-sauvegarder-la-configuration.md)
- [Find where the server's data is](rip-savoir-ou-sont-les-donnees-du-serveur.md)
- [Monitor the server service](rip-surveiller-le-service-du-serveur.md)
