---
titre: Know what never goes through the cloud
description: The ERP bridge, job forwarding between two RIPs and linking to the cloud stay reserved for the workshop.
sous_rubrique: rip-cloud-premiers-pas
position: 22
mots_cles: limits, blocked through cloud, ERP bridge, forwarding, workshop, remote
slug: rip-cloud-know-what-never-goes-through-the-cloud
traduction_de: rip-cloud-ce-qui-reste-reserve-a-l-atelier
---

Some functions stay reserved for the workshop, even when the cloud is active.
This article lists them and explains the message you will see when working remotely.

## Before you start

- This rule protects the workshop: it cannot be turned off.
- It applies to every request that arrives through the cloud, whatever account is used.

## Steps

1. Identify what is reserved for the workshop:
   - the **ERP bridge** and its settings;
   - **forwarding jobs between two RIPs**;
   - **linking** and **unlinking** the cloud itself.
2. If you need one of them, sign in from the workshop: on the local network, with the server's local address.
3. If you are remote and see the message **Impossible à distance : à faire depuis l'atelier.** (*Not possible remotely: do this from the workshop.*), go back to the workshop or ask someone on site to do it.

## Good to know

- The server itself recognises what arrives through the cloud: a request from the local network cannot carry that mark.
- As a precaution, the server also refuses any request coming through the cloud when **no account** is declared on the server. See [Fix "the cloud requires at least one account"](../rip-cloud-depannage/rip-cloud-cloud-demande-un-compte.md).
- The **restore** order for a backup sent by the platform is an exception: the server accepts it only if it is signed. See [Restore a configuration backup](../rip-cloud-sauvegardes/rip-cloud-restaurer-une-sauvegarde.md).

## See also

- [Understand the cloud's error messages](../rip-cloud-depannage/rip-cloud-comprendre-les-messages-d-erreur.md)
- [Understand cloud security](rip-cloud-securite.md)
- [Access your RIP server remotely](rip-cloud-acceder-a-distance.md)
