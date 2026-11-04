---
titre: Download an online backup
description: Download a version of the configuration backup from the portal to keep a copy.
sous_rubrique: rip-cloud-sauvegardes
position: 20
mots_cles: download, backup, archive, configuration, copy
slug: rip-cloud-download-an-online-backup
traduction_de: rip-cloud-telecharger-une-sauvegarde
---

Each backup version can be downloaded from the portal.
This is useful before a restore or to keep an offline copy.

## Before you start

- The server must be linked to the cloud and have sent at least one backup.
- The backup contains the configuration (machines, settings, ICC profiles…), **neither jobs nor accounts**.

## Steps

1. Open **My sites** (*Mes sites*) on the portal.
2. Click **Backups** (*Sauvegardes*) under the site's name.
3. Find the version you want: date, reason (**Nightly** — *Nocturne*, **After a change** — *Après modification*…) and size.
4. Click **Download** (*Télécharger*) on its line.

   ![A site's Backups page with the Download column on each version](../../images/cloud-sauvegardes.png)

## Good to know

- The file is the same archive as the server's **System › Backup** (*Système › Sauvegarde*): you can load it on the server itself. See [Back up the server's configuration](../rip-administration/rip-sauvegarder-la-configuration.md).
- Backups are encrypted and hosted in France.
- Before a restore, if in doubt, first download a backup of the current state. See [Restore a configuration backup](rip-cloud-restaurer-une-sauvegarde.md).
- If the list is empty or old, see [Understand backup retention and space](rip-cloud-duree-de-conservation.md).

## See also

- [Understand online backups](rip-cloud-comprendre-les-sauvegardes.md)
- [Restore a configuration backup](rip-cloud-restaurer-une-sauvegarde.md)
- [Rebuild a server from a backup](../rip-administration/rip-remonter-un-serveur.md)
