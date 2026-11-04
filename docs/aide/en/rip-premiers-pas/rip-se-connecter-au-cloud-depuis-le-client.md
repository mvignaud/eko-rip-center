---
titre: Connect to the cloud from the Mac or Linux client
description: Reach your server from outside the workshop with the Connect to the cloud button of the Mac and Linux clients.
sous_rubrique: rip-premiers-pas
position: 71
mots_cles: cloud, client, mac, linux, remote, platform, notifications, sign out
slug: rip-connect-to-the-cloud-from-the-mac-or-linux-client
traduction_de: rip-se-connecter-au-cloud-depuis-le-client
---

The cloud lets you reach the workshop server from outside, with no VPN. The Mac and Linux clients open your platform account in the browser, then list your sites.

## Before you start

The server is already linked to the platform (see [Link the server to the Cloud](../rip-cloud-premiers-pas/rip-cloud-relier-un-serveur.md)) and you have a platform account. Allow 2 minutes.

## Steps

1. On the connection screen, choose **Connect to the cloud** (*Se connecter au cloud*) (on Linux: **Cloud › Connect to the cloud** menu, *Cloud › Se connecter au cloud*).
2. The browser opens on the platform. Sign in and authorise the application.
3. Go back to the client: it lists the sites of your account, with your role. An offline site is greyed out.
4. Click the site you want. No RIP password is asked.

## Good to know

- The client reopens the last site the next time it starts.
- **Sign out of the cloud** (*Se déconnecter du cloud*, a button on the sites screen or the **Server** (*Serveur*) menu on Mac; on Linux, the sites screen or the **Sign out** (*Se déconnecter*) menu on a cloud site) revokes access. You can also disconnect apps from your account on the portal (see [Disconnect an app from the cloud](../rip-cloud-compte/rip-cloud-deconnecter-les-apps.md)).
- On Linux, if the browser does not open, the authorisation address is displayed: copy it into your browser. The client waits for the return for 5 minutes at most; **Cancel** (*Annuler*) abandons the operation.
- On Mac, the access token is kept in the Keychain; on Linux, in a configuration file in your home folder reserved to your user.
- **Administration…** is not yet available remotely: use it from the workshop network.
- As long as the account is connected, even on a local server, the client checks the platform every minute and shows a system notification: site offline, back online, new app connected. Clicking the notification opens the site concerned. macOS asks for permission right after the first sign-in. On Linux, an alert announces it at the first sign-in, and the **Cloud notifications** (*Notifications du cloud*) checkbox in the main menu, on by default, turns them off.
- You can also enter the site address directly, for example `https://atelier.platform.e-ko.rip`, then sign in with your server account.
- The Windows client does not have this button: enter the site address.

## See also

- [Connect a Mac, Linux or mobile app to the cloud](../rip-cloud-premiers-pas/rip-cloud-connecter-une-app.md)
- [Connect to the server from a workstation](rip-se-connecter-au-serveur.md)
- [Turn on cloud notifications](../rip-cloud-compte/rip-cloud-activer-les-notifications.md)
