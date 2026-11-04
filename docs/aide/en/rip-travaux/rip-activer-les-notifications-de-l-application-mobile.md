---
titre: Turn on and set up mobile app notifications
description: Allow notifications, test them and choose between machine errors and print completions in the Settings tab.
sous_rubrique: rip-travaux
position: 74
mots_cles: notifications, mobile app, settings, machine errors, print completions, test
slug: rip-turn-on-mobile-app-notifications
traduction_de: rip-activer-les-notifications-de-l-application-mobile
---

Notifications alert you even when the app is closed: machine in error, ink almost empty, output failed.

## Before you start

The app is connected to the server, and the RIP server has Internet access: it is the server that sends the machine notifications.

## Steps

1. Accept the permission your phone asks for the first time you connect.
2. Open the **Settings** (*Réglages*) tab and check that **Notifications** (*Notifications*) is on.
3. Choose what you receive: **Machine errors** (*Erreurs machine*: alerts, ink, failures, unreachable machine) and, if you want, **Print completions** (*Fins d'impression*: one notification each time a print finishes).
4. Tap **Send a test notification** (*Envoyer une notification d'essai*): it should arrive within a few seconds.

## Good to know

- **Print completions** is off by default.
- The **Settings** tab also shows the server, the user and the role (**Administrator** or **Operator**), the **Sign out** button, the app version number and the **Help / Documentation** link.
- After a server restart, no burst of notifications is sent: the server simply picks up the current state.
- When connected to the cloud, the app also receives platform notifications (site offline, site back online, new app connected). A phone stays registered as long as you open the app at least once every 30 days.
- Nothing arrives: see [I am not receiving mobile app notifications](../rip-depannage/rip-notifications-mobile-absentes.md).

## See also

- [Understand the events the mobile app notifies](rip-comprendre-les-evenements-notifies.md)
- [View the mobile app alerts](rip-consulter-les-alertes-de-l-application-mobile.md)
- [Turn on cloud notifications](../rip-cloud-compte/rip-cloud-activer-les-notifications.md)
