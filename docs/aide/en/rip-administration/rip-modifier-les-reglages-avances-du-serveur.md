---
titre: Change the server's advanced settings
description: Change the port, data folder, retention period, free-space thresholds and the server's other settings in a file.
sous_rubrique: rip-administration
position: 21
mots_cles: advanced settings, variables, eko_rip, port, retention, threshold, snmp, plotter, announcement, default file
slug: rip-change-the-servers-advanced-settings
traduction_de: rip-modifier-les-reglages-avances-du-serveur
---

The server's advanced settings are given as variables in a single file. This article explains how to change them and gives the full list, with their default values.

## Before you start

You have a terminal on the server and `sudo` access. Restarting the service interrupts the render in progress: choose a quiet moment.

## Steps

1. Open the settings file: `sudo nano /etc/default/eko-rip-serveur`.
2. Add or change one line per setting, in the form `EKO_RIP_CONSERVATION_JOURS=14`.
3. Save, then restart the service: `sudo systemctl restart eko-rip-serveur`.
4. Check in the client's **System** (*Système*) view that the value is taken into account (retention period, free space).

### Common settings

| Setting | Default | Role |
|---|---|---|
| `EKO_RIP_ADDR` | all interfaces, port `8640` | listening address and port |
| `EKO_RIP_DATA` | `eko-rip-data` (package: `/var/lib/eko-rip`) | data and jobs folder |
| `EKO_RIP_GS` | `gs` | Ghostscript command |
| `EKO_RIP_MAX_MO` | `8192` | maximum size of a received file (PDF, SVG, TIFF, PNG, JPEG), in MB |
| `EKO_RIP_DELAI_S` | `1800` | maximum duration of a render, in seconds |
| `EKO_RIP_RENDUS` | one per core, 4 at most | renders run at the same time (1 to 16); each takes a core and memory, lower it on a small computer |
| `EKO_RIP_CONSERVATION_JOURS` | `30` | jobs deleted after this number of days (0: never) |
| `EKO_RIP_SEUIL_LIBRE_MO` | `5120` | free space to keep, in MB: below it, the oldest inactive jobs are purged and a job that would not fit is refused (reason `espace_insuffisant`); a drop into a hot folder becomes a job in error with this reason; 0: no threshold |
| `EKO_RIP_SEUIL_ALERTE_MO` | `20480` | low threshold, in MB: below it, the "disque presque plein" (disk almost full) alert (`espace_disque_bas`) is added to every new job and the API reports `disque_niveau: bas`; 0: none |
| `EKO_RIP_PURGER_ERP_FAITS` | `0` | `1`: a job that came from the ERP is deleted as soon as it is done (one output finished, none waiting or failed), without waiting for the retention period |
| `EKO_RIP_PLOTTER` | `/dev/kona` | port of the USB cutter |
| `EKO_RIP_PLOTTER_STTY` | `9600 cs8 -cstopb -parenb crtscts raw -echo` | cutter port settings |
| `EKO_RIP_EPSON_SNMP` | (empty) | Epson address: width of the loaded media read by SNMP |
| `EKO_RIP_EPSON_COMMUNAUTE` | `public` | SNMP community of the Epson |
| `EKO_RIP_EXPO_JETON` | (empty) | access token of the mobile app's notification service, if "enhanced push security" is turned on for its project |
| `EKO_RIP_ANNONCE` | `1` | `0`: do not announce itself on the local network (the clients then cannot find the server by themselves, so the address must be entered) |
| `EKO_RIP_CLOUD_NOM` | PC name | name suggested for the site on the Cloud |
| `EKO_RIP_LENTEUR_MIN` | `30` | duration, in minutes, beyond which a render is flagged as slow in the automatic reports |
| `EKO_TELEMETRIE` | on | `0`: stops sending the anonymous installation statistics |
| `EKO_RIP_ACCES_NAVIGATEUR` | `0` | `1`: full interface in a browser too (development only) |

## Good to know

- The package creates this file; changes survive updates.
- The render timeout (`EKO_RIP_DELAI_S`) quickly tells you whether the computer is too weak: a render that exceeds 30 minutes fails.
- The maximum file size applies on sending: beyond it, the client shows "Fichier trop volumineux pour le serveur." (file too large for the server). Raise `EKO_RIP_MAX_MO` then restart.
- Below 5 GB free by default, the server itself deletes the oldest inactive jobs: raise the threshold if you want more margin.
- To know where each data file is stored, see [Find where the server's data is](rip-savoir-ou-sont-les-donnees-du-serveur.md).

## See also

- [Monitor the server service](rip-surveiller-le-service-du-serveur.md)
- [Choose the server computer](../rip-premiers-pas/rip-choisir-le-poste-serveur.md)
- [Fix a failed render or a rejected file](../rip-depannage/rip-un-rendu-echoue.md)
- [Understand the anonymous statistics](rip-comprendre-les-statistiques-anonymes.md)
