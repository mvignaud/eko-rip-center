---
titre: Find where the server's data is
description: Locate each of the server's data files (machines, media, accounts, queue, jobs, backups) and what the backup contains.
sous_rubrique: rip-administration
position: 22
mots_cles: data, folder, var lib eko-rip, files, json, jobs, backups, location
slug: rip-find-where-the-servers-data-is
traduction_de: rip-savoir-ou-sont-les-donnees-du-serveur
---

Everything the server keeps is in a single folder: `/var/lib/eko-rip` for the package (elsewhere, the folder given by `EKO_RIP_DATA`). This article describes the contents of that folder for the administrator who backs up or troubleshoots.

## Before you start

You have a terminal on the server and `sudo` access. Do not edit these files by hand: use the client.

## Steps

1. List the folder: `sudo ls /var/lib/eko-rip`.
2. Find the file you are looking for in the table.

   | Path | Contents |
   |---|---|
   | `machines.json` | machine catalogue |
   | `tarifs.json` | ink and media prices, excl. or incl. tax |
   | `reglages.json` | saved presets |
   | `supports.json` | media per machine and their print modes (stable identifiers); taken from `tarifs.json` at first start |
   | `charges.json` | media loaded in each machine, entered by the operator or read from the Epson |
   | `decoupe.json` | the server's cutting settings (prefixes, non-printable layers, open paths, sorting, after cutting) |
   | `tons_directs.json` | spot colour table (exact name, aliases, Lab colour) |
   | `tons_inconnus.json` | spot colours found but missing from the table (number of jobs, last time), listed in **System › Spot colours** (*Système › Tons directs*) |
   | `utilisateurs.json` | accounts (hashed passwords, permissions 0600) |
   | `erp.json` | ERP bridge: token fingerprint, HMAC secret, ERP URL, activation, instance (permissions 0600) |
   | `cloud.json` | E-KO RIP Cloud: platform address, site and key received at pairing (permissions 0600) |
   | `pont/` | ERP bridge: event log (`journal.jsonl`), sending status (`etat.json`) and received files; this is not configuration, it is not backed up |
   | `file.json` | output queue and history |
   | `profils/` | uploaded ICC profiles |
   | `entrees/<preset>/` | hot folders, one per preset |
   | `travaux/<id>/` | one folder per job: source, `job.json`, plates (`prod/`), previews |
   | `sorties/<machine>/` | plates copied for a machine with a Folder connection (Epson), cut files of a cutter in Folder mode |
   | `releves/<machine>.jsonl` | SNMP readings (inks, roll) for actual consumption |
   | `sauvegardes/avant-<version>-<date>.tar.gz` | configuration copied before each package update (machines, media, presets, rates, accounts, ERP bridge, profiles; 10 kept, permissions 0600) |
   | `*.json.prec` | previous version of each status file, kept at each write |
   | `*.json.illisible` | copy of a status file found damaged after a power cut |
3. To back up, follow [Back up the server configuration](rip-sauvegarder-la-configuration.md): the downloaded file contains the machine catalogue, the rates, the saved presets, cutting, spot colours and ICC profiles.

## Good to know

- Jobs older than `EKO_RIP_CONSERVATION_JOURS` days (30 by default) are deleted with their outputs; 0 keeps them. See [Change the server's advanced settings](rip-modifier-les-reglages-avances-du-serveur.md).
- The accounts file is not in the downloaded backup: copy `utilisateurs.json` separately or recreate the accounts.
- Jobs are large (plates at full resolution) and can be redone from their source files.
- Each status file keeps its previous version (`.prec`): see [Recover after a power cut](rip-reprendre-apres-une-coupure-de-courant.md).

## See also

- [Back up the server configuration](rip-sauvegarder-la-configuration.md)
- [Rebuild a server from a backup](rip-remonter-un-serveur.md)
- [Recover after a power cut](rip-reprendre-apres-une-coupure-de-courant.md)
- [Share the hot folders](rip-partager-les-dossiers-surveilles.md)
