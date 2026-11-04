---
titre: Fix the client not finding the server
description: What to do when the server list stays empty or the connection fails: checks to run and how to enter the address.
sous_rubrique: rip-depannage
position: 1
mots_cles: server not found, discovery, local network, firewall, mac, address, port 8640
slug: rip-fix-client-cannot-find-server
traduction_de: rip-client-ne-trouve-pas-le-serveur
---

The client lists the servers on the local network under **Servers found on the network** (*Serveurs trouvés sur le réseau*). If yours is not there, run these checks in order.

## Before you start

You are on the workshop network. If you have access to the server, open a terminal on it.

## Steps

1. Check that the server is running: `systemctl status eko-rip-serveur` on the server.
2. From your workstation, open `http://<server-address>:8640` in a browser. The status page should appear.

   ![Server status page, showing its version, its machines and the address to enter in the client](../../images/serveur-page-etat.png)
3. On Mac, allow the client to access the **local network**: System Settings › Privacy & Security › Local Network.
4. In the client, click **Search again** (*Rechercher à nouveau*).
5. Still nothing: enter the address under **Enter an address** (*Saisir une adresse*), in the form `<server-address>:8640`.
6. If a firewall is running on the server, allow TCP port 8640 and UDP port 5353 (discovery).

## Good to know

- Automatic discovery does not cross routers, VPNs or some guest Wi-Fi networks.
- On the server, the `EKO_RIP_ANNONCE=0` variable turns off the server's announcement on the network: if it is set, the server never appears in the list. `EKO_RIP_CLOUD_NOM` sets the name it announces itself under (by default, the machine name).
- The Windows client does not search for servers yet: you always have to enter the address.
- The browser only shows a status page: this is intentional. For the full interface, use the client.
- Never open port 8640 to the Internet: the connection is not encrypted. To work remotely, use the Cloud.

## See also

- [Connect to the server from a workstation](../rip-premiers-pas/rip-se-connecter-au-serveur.md)
- [Connection refused](rip-connexion-refusee.md)
- [Link the server to the Cloud](../rip-cloud-premiers-pas/rip-cloud-relier-un-serveur.md)
- [Connect to the cloud from the Mac or Linux client](../rip-premiers-pas/rip-se-connecter-au-cloud-depuis-le-client.md)
