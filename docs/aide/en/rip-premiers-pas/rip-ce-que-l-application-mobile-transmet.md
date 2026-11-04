---
titre: Understand what the mobile app sends
description: Learn where your credentials and notification token go, and what the publisher receives, when you use the E-KO RIP mobile app.
sous_rubrique: rip-premiers-pas
position: 76
mots_cles: privacy, mobile app, data, credentials, token, notifications
slug: rip-understand-what-the-mobile-app-sends
traduction_de: rip-ce-que-l-application-mobile-transmet
---

The E-KO RIP app contains no advertising and no usage tracking. This article summarises what it sends and to whom.

## Before you start

Nothing to prepare: this article is a reference.

## Steps

1. Without a cloud connection, the app talks only to the workshop's RIP server: the publisher receives nothing.
2. With the cloud, the E-KO RIP Cloud platform knows your account, the phone's name and, if notifications are on, its notification token.
3. Your credentials stay in the phone's secure storage and are sent only to the server you chose.
4. For notifications, a token from the phone is sent to the RIP server (or to the platform if you connect through it). Messages then travel through a push notification service, then through Apple or Google.
5. To remove this data from the phone, tap **Sign out** (*Se déconnecter*) in the **Settings** (*Réglages*) tab.

## Good to know

- Signing out deletes the credentials from the phone and removes the phone from the notification list.
- A phone whose account is deleted no longer receives anything.
- The full policy is published at https://e-ko.rip/app/confidentialite/.

## See also

- [Install the E-KO RIP mobile app](rip-installer-l-application-mobile.md)
- [Turn on and adjust the mobile app notifications](../rip-travaux/rip-activer-les-notifications-de-l-application-mobile.md)
- [Connect to the server from outside with the mobile app](rip-connecter-l-application-mobile-via-le-cloud.md)
