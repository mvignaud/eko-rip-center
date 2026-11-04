---
titre: Understand roles on a site
description: Owner, administrator, operator, read-only: what each role can do on a cloud site.
sous_rubrique: rip-cloud-compte
position: 24
mots_cles: roles, owner, administrator, operator, read-only, rights
slug: rip-cloud-understand-roles-on-a-site
traduction_de: rip-cloud-roles-sur-un-site
---

Your role on a site decides what you can do from the portal and from the apps.
The **Role** column (*Rôle*) in **My sites** (*Mes sites*) shows it for each site.

## Before you start

- The account that links a server becomes its **owner**.
- Sharing a site with other accounts (colleagues, contractors) is not offered yet: for now, the owner is the account that linked the server.

## Steps

1. Open **My sites**.
2. Read the **Role** column: **Owner** (*Propriétaire*), **Administrator** (*Administrateur*), **Operator** (*Opérateur*) or **Read-only** (*Lecture*).
3. Refer to the table.

   | Role | In an app connected to the cloud | On the portal |
   |---|---|---|
   | Owner | RIP administrator | Removes the site, restores backups, receives alerts |
   | Administrator | RIP administrator | Restores backups, receives alerts |
   | Operator | RIP operator | Sees the site |
   | Read-only | Read-only operator: no writing | Sees the site |

   ![My sites list with the Role column](../../images/cloud-mes-sites.png)

## Good to know

- Only the **owner** can remove a site from the portal. See [Unlink a RIP server from the cloud](rip-cloud-detacher-un-serveur.md).
- The owner and the administrators receive offline-site alerts and can restore a backup.
- A **Read-only** account that tries to make a change sees the message "Votre rôle sur ce site (cloud) est en lecture seule." (your role on this site (cloud) is read-only).
- These roles apply to the cloud only. RIP server accounts have their own rights: see [Manage the server's accounts](../rip-administration/rip-gerer-les-comptes.md).

## See also

- [Connect an app to the cloud](../rip-cloud-premiers-pas/rip-cloud-connecter-une-app.md)
- [View and manage your account](rip-cloud-gerer-son-compte.md)
- [Restore a configuration backup](../rip-cloud-sauvegardes/rip-cloud-restaurer-une-sauvegarde.md)
