---
titre: Connect to the server from a workstation
description: Choose your server from the local network list, or enter its address, then open your session in the client.
sous_rubrique: rip-premiers-pas
position: 6
mots_cles: connection, server, address, discovery, session, change server
slug: rip-connect-to-the-server-from-a-workstation
traduction_de: rip-se-connecter-au-serveur
---

At first launch, the client needs to know which server to talk to. This article explains how to choose it, and how to change it later.

## Before you start

The client is installed and the server is running. If the server has accounts, have your user name and password ready.

## Steps

1. Start the client. The connection screen searches for servers on the local network by itself.
2. In **Servers found on the network** (*Serveurs trouvés sur le réseau*; name, address, version), click your server.
3. If nothing appears, click **Search again** (*Rechercher à nouveau*) and wait about 3 seconds.
4. Still nothing: enter the address under **Enter an address** (*Saisir une adresse*), in the form `<server-address>:8640`.
5. If the server has accounts, enter your name and password.

## Good to know

- The client remembers the server. To change it, use **Change server…** (*Changer de serveur…*; on Mac: <kbd>Cmd</kbd> + <kbd>Shift</kbd> + <kbd>S</kbd>). The list offers the servers found and the last address entered.
- A client talks to only one server at a time.
- The session lasts 12 hours.
- Automatic discovery crosses neither routers nor VPNs: enter the address instead.
- Discovery uses the local network announcement service (DNS-SD, UDP port 5353). On Mac, macOS asks for permission to access the local network at first launch. The server announces itself under the machine's name.
- A browser opened on the server's address shows only a status page (version, machines, address to enter in the client): the interface is reserved for the client.
- From outside the workshop, enter the site address on the Cloud, or use the **Connect to the cloud** (*Se connecter au cloud*) button (Mac and Linux clients).

## See also

- [The client cannot find the server](../rip-depannage/rip-client-ne-trouve-pas-le-serveur.md)
- [Link the server to the Cloud](../rip-cloud-premiers-pas/rip-cloud-relier-un-serveur.md)
- [Understand the client window and its menus](rip-comprendre-la-fenetre-du-client.md)
- [Create the server accounts](../rip-administration/rip-creer-les-comptes.md)
- [Connect to the cloud from the Mac or Linux client](rip-se-connecter-au-cloud-depuis-le-client.md)
