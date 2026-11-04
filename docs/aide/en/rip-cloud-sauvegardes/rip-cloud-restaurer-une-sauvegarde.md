---
titre: Restore a configuration backup
description: Replace the RIP server's configuration with an online backup's, or restore to another site with the Pro plan.
sous_rubrique: rip-cloud-sauvegardes
position: 2
mots_cles: restore, backup, configuration, another site
slug: rip-cloud-restore-a-configuration-backup
traduction_de: rip-cloud-restaurer-une-sauvegarde
---
Restoring puts the RIP server's configuration back to the state of a backup.
The site's owner and administrators can do it from the portal.

## Before you start

- A restore replaces the server's current configuration.
- If in doubt, download a backup of the current state first.

## Steps

1. Open **My sites** (*Mes sites*), then click **Backups** (*Sauvegardes*) under the site's name.
2. Find the version you want: date, reason and size.
3. If needed, first click **Download** (*Télécharger*) to keep a copy.
4. On the version's line, tick **Replace the current settings** (*Remplacer les réglages actuels*).
5. Click **Restore** (*Restaurer*), then confirm in the window that opens.

   ![A site's Backups page: three versions with their date, reason, size, the Download link, the Replace the current settings box and the Restore button](../../images/cloud-sauvegardes.png)

## Good to know

- With the **Pro** plan, a backup can also be restored to **another site** of the same owner.
- See [Restore a backup to another site](rip-cloud-restaurer-vers-un-autre-site.md) for that use.
- The backup contains neither jobs nor accounts: after a restore, the server's accounts do not change.
- The server receives the order through its connection with the platform, checks that it really comes from there (signed order, valid for a few minutes, usable once only), downloads the archive and applies it. An order without a valid signature is refused with the message "Ordre du cloud refusé : signature absente, fausse ou périmée." (cloud order refused: signature missing, wrong or expired).
- A "Backup restored" e-mail warns the owner and the administrators.
- The server must be online: see [Fix an offline site](../rip-cloud-depannage/rip-cloud-site-hors-ligne.md).

## See also

- [Understand online backups](rip-cloud-comprendre-les-sauvegardes.md)
- [Space and failure alerts](rip-cloud-alertes-de-sauvegarde.md)
