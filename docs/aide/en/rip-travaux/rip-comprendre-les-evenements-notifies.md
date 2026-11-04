---
titre: Understand the events the mobile app notifies
description: Table of the events that trigger a notification: machine alert, ink, empty cartridge, unreachable machine, failures.
sous_rubrique: rip-travaux
position: 75
mots_cles: notifications, events, ink, unreachable, failure, threshold
slug: rip-understand-mobile-app-notification-events
traduction_de: rip-comprendre-les-evenements-notifies
---

This article lists what triggers a notification on your phone, and when.

## Before you start

Notifications are turned on (see [Turn on and set up mobile app notifications](rip-activer-les-notifications-de-l-application-mobile.md)).

## Steps

1. Find the event in the table.
2. Read the trigger condition.
3. Act on site if needed: change the media, replace a cartridge, check the machine's network.

| Event | When |
|---|---|
| Machine alert | A new alert appears (cover open, media out...). Ordinary states (ready, printing, heating, starting, sleep) do not notify. |
| Ink almost empty | An ink drops to 5% or less. |
| Empty cartridge | An ink reaches 0%. |
| Machine unreachable | Three readings in a row with no reply, then a notification when it answers again. |
| Output failed | A print or a cut fails. |
| Render failed | A job's render fails (a single notification per job). |
| Print finished | Only if **Print completions** (*Fins d'impression*) is on. |

## Good to know

- The alert is shown in French; a message the RIP does not know yet stays in the machine's language.
- A phone whose account has been deleted no longer receives anything.
- Notifications are also kept in the **Alerts** (*Alertes*) tab.

## See also

- [Turn on and set up mobile app notifications](rip-activer-les-notifications-de-l-application-mobile.md)
- [View the mobile app alerts](rip-consulter-les-alertes-de-l-application-mobile.md)
- [A render fails](../rip-depannage/rip-un-rendu-echoue.md)
