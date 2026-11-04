---
titre: Check that the online backup is working
description: Check the date of the latest backup and its status (ok, plan, quota, error) on the portal and on the server.
sous_rubrique: rip-cloud-sauvegardes
position: 23
mots_cles: status, latest backup, quota, error, check, weekly summary
slug: rip-cloud-check-that-the-online-backup-is-working
traduction_de: rip-cloud-lire-l-etat-de-la-sauvegarde
---

A useful backup is a recent backup.
Here is how to check in a few seconds that yours is.

## Before you start

- The server must be linked to the cloud, with a plan that includes backups.
- The site's owner and administrators receive the alerts.

## Steps

1. Open **My sites** (*Mes sites*), then **Backups** (*Sauvegardes*) under the site's name.
2. Look at the first line of the list: the date should be recent. Every night, a backup is sent if the configuration changed.
3. On the server, open **System › E-KO RIP Cloud** (*Système › Cloud E-KO RIP*) to check that the status is **Connected: reachable remotely** (*Connecté : accessible à distance*). See [Read the cloud status on the server](../rip-cloud-premiers-pas/rip-cloud-lire-l-etat-du-cloud-sur-le-serveur.md).
4. Every Monday, read the weekly summary: it gives the backup status of each site.

## Good to know

- The backup status on the server side is one of: **ok**, **plan** (the plan does not include backups), **quota** (no more space) or **error** (with a message).
- If no nightly backup has been received for more than two days, an e-mail warns the owner and the administrators. See [Understand backup space and failure alerts](rip-cloud-alertes-de-sauvegarde.md).
- The two most common causes are an offline site and a full quota. See [Fix an offline site](../rip-cloud-depannage/rip-cloud-site-hors-ligne.md).

## See also

- [Understand online backups](rip-cloud-comprendre-les-sauvegardes.md)
- [Understand backup retention and space](rip-cloud-duree-de-conservation.md)
- [View the list of platform e-mails](../rip-cloud-compte/rip-cloud-comprendre-les-emails.md)
