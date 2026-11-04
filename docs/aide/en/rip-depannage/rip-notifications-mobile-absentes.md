---
titre: Fix missing mobile app notifications
description: What to check when the E-KO RIP app sends no notifications: permission, switch, server, phone registration.
sous_rubrique: rip-depannage
position: 70
mots_cles: notifications, mobile app, missing, permission, registration, internet
slug: rip-fix-missing-mobile-notifications
traduction_de: rip-notifications-mobile-absentes
---

You receive neither machine alerts nor test notifications. Run these checks in order.

## Before you start

The app is connected to the server.

## Steps

1. In the phone settings, check that notifications are allowed for the app.
2. In the app, open **Settings** (*Réglages*) and check that the **Notifications** switch is on.
3. Tap **Send a test notification** (*Envoyer une notification d'essai*).
4. Check that the RIP server is up to date and has Internet access: it is the server that sends the machine notifications.
5. Tap **Sign out** (*Se déconnecter*), then sign in again: this renews the phone's registration.

## Good to know

- On the server, if "enhanced push security" is enabled on the notification project, its access token goes in `EKO_RIP_EXPO_JETON` (file `/etc/default/eko-rip-serveur`). The server must be able to reach `https://exp.host` outbound.
- Ordinary machine states (ready, printing, heating, starting, standby) do not trigger notifications, and **Print completions** (*Fins d'impression*) is off by default.
- A phone stays registered with the cloud as long as the app is opened at least once every 30 days.
- Missed events remain readable in the **Alerts** tab (*Alertes*).

## See also

- [Turn on and set up mobile app notifications](../rip-travaux/rip-activer-les-notifications-de-l-application-mobile.md)
- [Understand the events notified by the mobile app](../rip-travaux/rip-comprendre-les-evenements-notifies.md)
- [View the mobile app alerts](../rip-travaux/rip-consulter-les-alertes-de-l-application-mobile.md)
