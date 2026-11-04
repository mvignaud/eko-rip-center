---
titre: Follow production from the mobile app
description: Connect the E-KO RIP app to your server, follow the queue, ink and alerts, and turn on notifications.
sous_rubrique: rip-travaux
position: 7
mots_cles: mobile app, iphone, android, notifications, alerts, queue, machines
slug: rip-follow-production-from-the-mobile-app
traduction_de: rip-suivre-la-production-sur-mobile
---

The E-KO RIP app is for monitoring: queue, machines, ink levels and alerts. It sends no files and does not set printing or cutting.

## Before you start

Your phone is on the workshop Wi-Fi. You have the server address and, if the server has accounts, a username and password (an **Operator** account is enough). No E-KO account is needed to install the app.

## Steps

1. Install the app from the page https://e-ko.rip/app/ then open it.
2. In the **Server** (*Serveur*) field, enter the server address: `http://<server-address>:8640`.
3. Tap **Continue** (*Continuer*).
4. If the server asks for an account, enter the username and password, then tap **Sign in** (*Se connecter*).
5. Open the **Queue** (*File*) tab to see each machine's outputs, in-progress ones first.
6. Open the **Machines** tab for the state, inks and remaining media.
7. Open the **Alerts** (*Alertes*) tab for the last 200 events.
8. In **Settings** (*Réglages*), check that **Notifications** is on, then tap **Send a test notification** (*Envoyer une notification d'essai*).

![App Queue tab, with each machine's in-progress and waiting outputs](../../images/mobile-file.png)

![App Machines tab, with each machine's state, inks and roll](../../images/mobile-machines.png)

## Good to know

- Each tab has its own article: [Queue](rip-suivre-la-file-sur-l-application-mobile.md), [Machines](rip-suivre-les-machines-sur-l-application-mobile.md), [Alerts](rip-consulter-les-alertes-de-l-application-mobile.md).
- To act on an output, tap it then choose the action: **Pause**, **Move to front**, **Stop**, **Start** or **Resume**, **Rerun** (*Pause*, *Passer en tête*, *Stop*, *Lancer* or *Reprendre*, *Relancer*).
- **Pause** and **Stop** only act on an output that has not yet gone to the machine. A print already sent cannot be stopped from the app.
- In **Settings**, **Machine errors** (*Erreurs machine*) warns of alerts, ink almost empty, failures and unreachable machines. **Print completions** (*Fins d'impression*) is off by default.
- Away from the workshop, tap **Sign in to the cloud** (*Se connecter au cloud*; see [Connect to the server from outside with the mobile app](../rip-premiers-pas/rip-connecter-l-application-mobile-via-le-cloud.md)).
- No notification: check the permission in the phone's settings, then the **Notifications** switch. Signing out then back in renews the registration.
- To try the app without a server, tap **Try demo mode** (*Essayer le mode démo*).

## See also

- [Manage a machine's queue](rip-gerer-la-file-d-une-machine.md)
- [Connect to the server from a workstation](../rip-premiers-pas/rip-se-connecter-au-serveur.md)
- [Install the E-KO RIP mobile app](../rip-premiers-pas/rip-installer-l-application-mobile.md)
- [Turn on and set up mobile app notifications](rip-activer-les-notifications-de-l-application-mobile.md)
