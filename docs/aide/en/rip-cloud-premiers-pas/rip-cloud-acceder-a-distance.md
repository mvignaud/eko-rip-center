---
titre: Access your RIP server remotely
description: Use your site's address in a desktop client, a browser or the mobile app.
sous_rubrique: rip-cloud-premiers-pas
position: 4
mots_cles: remote access, address, browser, client, mobile
slug: rip-cloud-access-your-rip-server-remotely
traduction_de: rip-cloud-acceder-a-distance
---
Every linked site has its own address, which works from anywhere without a VPN.
You can use it in a desktop client, a browser or the mobile app.

## Before you start

- The server must be linked to the cloud.
- Find the address, which looks like `https://<site>.platform.e-ko.rip`, in **System › E-KO RIP Cloud** (*Système › Cloud E-KO RIP*) or in **My sites** (*Mes sites*).
- Have a RIP server account at hand.

## Steps

In a desktop client:

1. Open the Mac, Linux or Windows client.
2. Enter the site's address in place of the server's local address.
3. Sign in with a RIP server account.

In a browser:

1. Open the site's address.
2. Sign in with a RIP server account.

In the mobile app:

1. Enter the site's address in the **Server** field.
2. Sign in with a RIP server account.

![My sites list with each site's remote address](../../images/cloud-mes-sites.png)

## Good to know

- The accounts used remotely are those of the RIP server. The platform account is used for the portal and for **Sign in to the cloud** (*Se connecter au cloud*).
- The administration interface works just as it does in the workshop.
- The ERP bridge and forwarding jobs between two RIPs stay reserved for the workshop. See [Know what never goes through the cloud](rip-cloud-ce-qui-reste-reserve-a-l-atelier.md).
- The Mac, Linux and mobile apps also offer **Sign in to the cloud**: no need to enter the address or a second password. See [Connect an app to the cloud](rip-cloud-connecter-une-app.md).
- The old form `https://platform.e-ko.rip/s/<site>` still exists for clients already installed, but a web page opened through it is isolated (no scripts). Always prefer the site's own address.
- If the address does not respond, the site may be offline or its access suspended: see [Fix an offline site](../rip-cloud-depannage/rip-cloud-site-hors-ligne.md).

## See also

- [Connect an app to the cloud](rip-cloud-connecter-une-app.md)
- [Sign in to the server from a workstation](../rip-premiers-pas/rip-se-connecter-au-serveur.md)
- [Connect to the server from outside with the mobile app](../rip-premiers-pas/rip-connecter-l-application-mobile-via-le-cloud.md)
