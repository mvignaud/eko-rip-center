---
titre: Understand online backups
description: What the backups sent to the platform contain, when they are sent and how long they are kept.
sous_rubrique: rip-cloud-sauvegardes
position: 1
mots_cles: backups, configuration, versions, quota
slug: rip-cloud-understand-online-backups
traduction_de: rip-cloud-comprendre-les-sauvegardes
---
The RIP server sends the platform a backup of its configuration.
It lets you recover your settings after an incident.

## Before you start

- The server must be linked to the cloud.
- The backup is the same archive as **System › Backup** (*Système › Sauvegarde*): machines, settings, ICC profiles.
- It contains **neither jobs nor accounts**. It is encrypted and hosted in France.

## Steps

1. Open **My sites** (*Mes sites*) on the portal.
2. Click **Backups** (*Sauvegardes*) under the site's name.
3. Look through the versions: date, reason (**Nightly** — *Nocturne*, **After a change** — *Après modification*…) and size. The header shows the plan and the space used.

   ![A site's Backups page with the plan, the space used and the list of versions](../../images/cloud-sauvegardes.png)

## Good to know

- A backup is sent every night if the configuration changed, about 2 minutes after a change, and on request by an administrator.
- Retention: latest version (Free), 30 versions (Per machine), 90 days (Atelier), 1 year (Pro). The most recent one is always kept.
- Versions can be downloaded and restored from this page: see [Download an online backup](rip-cloud-telecharger-une-sauvegarde.md).
- The retention and quota rules are in [Understand backup retention and space](rip-cloud-duree-de-conservation.md).
- If the backup is old, check that the site is online and that the quota is not full.

## See also

- [Restore a backup](rip-cloud-restaurer-une-sauvegarde.md)
- [Check that the online backup is working](rip-cloud-lire-l-etat-de-la-sauvegarde.md)
- [Choose a plan](../rip-cloud-abonnement/rip-cloud-choisir-une-offre.md)
