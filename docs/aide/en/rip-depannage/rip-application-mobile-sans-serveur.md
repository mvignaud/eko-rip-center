---
titre: Fix the mobile app when it cannot reach the server
description: Check the Wi-Fi and the address you entered, and use the cloud connection when you are away from the workshop.
sous_rubrique: rip-depannage
position: 71
mots_cles: mobile app, connection, address, wifi, cloud, unreachable
slug: rip-fix-mobile-app-cannot-reach-server
traduction_de: rip-application-mobile-sans-serveur
---

The app shows a server connection error. In most cases, the local address is being used away from the workshop.

## Before you start

You have the server address, the same one as in the desktop clients.

## Steps

1. If you use the local address, check that the phone is on the workshop Wi-Fi.
2. Check the address you entered, with no spaces: `http://<server-address>:8640`.
3. Away from the workshop, tap **Connect to the cloud** (*Se connecter au cloud*): the local address can only be reached on site.
4. If the problem persists, check that the server is running (see [The client cannot find the server](rip-client-ne-trouve-pas-le-serveur.md)).

## Good to know

- A rejected account is a different problem: see [Connection refused](rip-connexion-refusee.md).
- Without the cloud, you need a VPN to reach the server from outside.

## See also

- [Connect the mobile app to the workshop server](../rip-premiers-pas/rip-connecter-l-application-mobile-a-l-atelier.md)
- [Connect to the server from outside with the mobile app](../rip-premiers-pas/rip-connecter-l-application-mobile-via-le-cloud.md)
- [Connection refused](rip-connexion-refusee.md)
