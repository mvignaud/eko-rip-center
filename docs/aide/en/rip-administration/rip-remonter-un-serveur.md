---
titre: Rebuild a server from a backup
description: Reinstall the server on a new PC and restore the configuration from the backup file.
sous_rubrique: rip-administration
position: 4
mots_cles: restore, backup, new server, replacement, configuration, accounts
slug: rip-rebuild-a-server-from-a-backup
traduction_de: rip-remonter-un-serveur
---

When you change the server PC, the configuration backup saves you from setting everything up again. This article is for the administrator.

## Before you start

You have the configuration backup file (see [Back up the configuration](rip-sauvegarder-la-configuration.md)) and a PC ready for the server.

## Steps

1. Install the server on the new PC (see [Install the RIP server](../rip-premiers-pas/rip-installer-le-serveur.md)).
2. Create the administrator account (see [Create the accounts](rip-creer-les-comptes.md)).
3. In the client, connect to this new server.
4. Open **Administration…** (*Administration…*), **System** (*Système*) view, scroll down to the **Configuration backup** (*Sauvegarde de la configuration*) section, then click **Restore…** (*Restaurer…*).

   ![Configuration backup section with the Restore… button circled](../../images/rip-sauvegarde-restaurer.png)
5. Choose the backup file.
6. Recreate the operator accounts.
7. Check the list of machines in the **Machines** (*Machines*) section of the **System** view.

   ![Machines section of the System view, with each machine's card](../../images/interface-systeme-machines.png)

## Good to know

- Everything is checked before writing: a damaged file changes nothing.
- Jobs are not in the backup: they are redone from their source files.
- Reserve the same fixed address for the new server, or change server in each client with **Change server…** (*Changer de serveur…*).

## See also

- [Back up the server configuration](rip-sauvegarder-la-configuration.md)
- [Install the RIP server](../rip-premiers-pas/rip-installer-le-serveur.md)
- [Connect to the server from a workstation](../rip-premiers-pas/rip-se-connecter-au-serveur.md)
- [Find where the server's data is](rip-savoir-ou-sont-les-donnees-du-serveur.md)
