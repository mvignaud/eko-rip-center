---
titre: Connect the mobile app to the workshop server
description: Enter the RIP server address and your account in the E-KO RIP app, from the workshop Wi-Fi.
sous_rubrique: rip-premiers-pas
position: 73
mots_cles: mobile app, connection, address, server, wifi, operator account, session
slug: rip-connect-the-mobile-app-to-the-workshop-server
traduction_de: rip-connecter-l-application-mobile-a-l-atelier
---

This article explains how to link the E-KO RIP app to the server when you are in the workshop. Away from the workshop, go through the cloud.

## Before you start

The app is installed and the phone is on the workshop Wi-Fi network. You know the server address, the same as in the desktop clients. If the server has accounts, have a user name and password ready.

## Steps

1. Open the app.
2. In the **Server** (*Serveur*) field, enter the server address, with no spaces: `http://<server-address>:8640`.
3. Tap **Continue** (*Continuer*).
4. If the server asks for an account, enter the user name and password, then tap **Sign in** (*Se connecter*).
5. Allow notifications when the phone asks.

## Good to know

- An **Operator** (*Opérateur*) account is enough to follow the queue and act on the outputs. Accounts are managed in the server administration (see [Create the server accounts](../rip-administration/rip-creer-les-comptes.md)).
- The name and password are kept in the phone's secure storage. The app reuses them to reopen the session when it expires (after 12 hours or after a server restart).
- The local address can be reached only from the workshop. Elsewhere, see [Connect to the server from outside with the mobile app](rip-connecter-l-application-mobile-via-le-cloud.md).
- The role (**Administrator** (*Administrateur*) or **Operator**) is shown in the **Settings** (*Réglages*) tab.
- If the app cannot reach the server, see [The mobile app cannot reach the server](../rip-depannage/rip-application-mobile-sans-serveur.md).

## See also

- [Install the E-KO RIP mobile app](rip-installer-l-application-mobile.md)
- [Follow production from the mobile app](../rip-travaux/rip-suivre-la-production-sur-mobile.md)
- [Connect to the server from a workstation](rip-se-connecter-au-serveur.md)
