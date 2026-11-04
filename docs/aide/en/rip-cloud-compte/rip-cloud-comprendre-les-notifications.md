---
titre: Understand cloud notifications
description: The three notified events (offline, back online, new app), where they arrive and what controls them.
sous_rubrique: rip-cloud-compte
position: 22
mots_cles: notifications, push, browser, apps, offline, log
slug: rip-cloud-understand-cloud-notifications
traduction_de: rip-cloud-comprendre-les-notifications
---

In addition to e-mails, the platform notifies the same events in your browser and in the apps.
This article explains what is notified and where.

## Before you start

- Notifications follow the **Receive alerts** (*Recevoir les alertes*) box in **My account** (*Mon compte*).
- They go to the site's owners and administrators.

## Steps

1. Identify the notified events:
   - **site offline**, after 15 minutes without a response from the server;
   - **site back online**, only if the "offline" alert was sent;
   - **new app connected**.
2. Choose where to receive them:
   - **in the browser**: see [Enable cloud notifications](rip-cloud-activer-les-notifications.md);
   - **in the Mac, Linux, Windows and mobile apps**: accept the permission to show notifications, requested right after the first sign-in to the cloud.
3. To stop on one browser, see [Stop notifications on a browser](rip-cloud-retirer-un-navigateur.md).

## Good to know

- The **new app connected** notification is always sent, even if **Receive alerts** is unticked: it is a security measure.
- Notifications go out even when e-mail sending is not set up.
- The desktop apps (Mac, Linux, Windows) check the platform's log every minute. This log keeps notifications for 30 days; on first launch, an app does not catch up on old ones.
- On mobile, the notification arrives even with the app closed. The phone is tied to the app's access: it disappears when the app is disconnected, when access is revoked in **My account** or when the account is deleted. A device that reconnects is notified only once.
- In the browser, notifications arrive even when the portal page is closed, as long as the browser is running. At most ten browsers can be registered per account.
- If permission was refused in the browser or in the app, restore it in the browser, system or phone settings, then reload the page.
- If phone notifications do not arrive, see [I am not receiving the mobile app's notifications](../rip-depannage/rip-notifications-mobile-absentes.md).

## See also

- [Enable cloud notifications](rip-cloud-activer-les-notifications.md)
- [View the list of platform e-mails](rip-cloud-comprendre-les-emails.md)
- [Fix an offline site](../rip-cloud-depannage/rip-cloud-site-hors-ligne.md)
