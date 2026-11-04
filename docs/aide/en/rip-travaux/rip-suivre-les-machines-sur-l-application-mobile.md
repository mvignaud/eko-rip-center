---
titre: Follow machine status on the mobile app
description: Read the Machines tab: each machine's state, current output, ink levels, remaining media and alerts.
sous_rubrique: rip-travaux
position: 72
mots_cles: mobile app, machines, ink, roll, unreachable, alerts, snmp
slug: rip-follow-machines-on-the-mobile-app
traduction_de: rip-suivre-les-machines-sur-l-application-mobile
---

The **Machines** tab shows one card per machine, so you know whether it can produce and whether it is short of ink or media.

## Before you start

The app is connected to the server.

## Steps

1. Open the **Machines** tab.
2. Read the card's state: **Printing** (*En impression*), **Cutting** (*En découpe*), **Waiting** (*En attente*) or **Unreachable** (*Injoignable*).
3. Find the output in progress.
4. Check the level of each ink: low levels are highlighted.
5. Read the media length remaining on the roll.
6. Read the current alerts (cover open, end of roll...) and the time of the last reading.

![Machines tab: each machine's state, inks and roll](../../images/mobile-machines.png)

## Good to know

- The server polls Epson machines about once a minute, so values are at most a few moments behind.
- An "Unreachable" machine no longer answers the server: check its power and its network on site.

## See also

- [Follow the print queue on the mobile app](rip-suivre-la-file-sur-l-application-mobile.md)
- [View the mobile app alerts](rip-consulter-les-alertes-de-l-application-mobile.md)
- [Choose a machine's connection](../rip-machines/rip-choisir-la-connexion-d-une-machine.md)
