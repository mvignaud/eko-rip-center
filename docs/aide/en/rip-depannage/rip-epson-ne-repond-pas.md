---
titre: Fix an Epson that does not respond or reports no media width
description: Check the SNMP address, the community and the network when the Epson reports neither its status nor the width of the loaded media.
sous_rubrique: rip-depannage
position: 25
mots_cles: epson, snmp, community, media width, unreachable, 9100, alert, direct driver
slug: rip-fix-unresponsive-epson
traduction_de: rip-epson-ne-repond-pas
---

The server reads the Epson's status over SNMP: loaded media, ink levels, alerts. This article helps when that information is missing.

## Before you start

You are the server administrator, with a terminal and `sudo` access.

## Steps

1. Check that the Epson is switched on and on the same network as the server.
2. Check the machine's SNMP address: `EKO_RIP_EPSON_SNMP` in `/etc/default/eko-rip-serveur`.
3. Check the community: `EKO_RIP_EPSON_COMMUNAUTE`, `public` by default.
4. From the server, test the print port: `nc -vz <printer-address> 9100`.
5. Restart the service: `sudo systemctl restart eko-rip-serveur`.
6. Check whether the **System** view (*Système*), **Media loaded in the Epson** section (*Média chargé dans l'Epson*), shows the media again.

## Good to know

- Without SNMP, the direct driver cannot stop a print on a printer alert. The server speaks SNMP itself: there is nothing to install.
- If the alert "L'imprimante signale une alerte : impression arrêtée." ("The printer reports an alert: printing stopped.") appears, clear it on the Epson, then click **Redo** (*Refaire*): see [The output to the machine failed](rip-sortie-vers-la-machine-echouee.md).
- A loaded media that cannot be read can also be chosen by hand by the operator.

## See also

- [Connect the Epson SureColor SC-S80600](../rip-machines/rip-brancher-epson-sc-s80600.md)
- [The output to the machine failed](rip-sortie-vers-la-machine-echouee.md)
- [Change the server's advanced settings](../rip-administration/rip-modifier-les-reglages-avances-du-serveur.md)
