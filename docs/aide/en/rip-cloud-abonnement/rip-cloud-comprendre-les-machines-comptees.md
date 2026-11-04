---
titre: Understand the machines counted in your plan
description: See how the server declares its machines to the platform and read the Usage table on the Subscription page.
sous_rubrique: rip-cloud-abonnement
position: 25
mots_cles: machines counted, inventory, printer, cutter, usage, quota
slug: rip-cloud-understand-the-machines-counted-in-your-plan
traduction_de: rip-cloud-comprendre-les-machines-comptees
---

Plans are paid per machine: the platform counts the ones your RIP server declares.
There is nothing to enter by hand.

## Before you start

- The server must be linked to the cloud and up to date.
- Free plan: 1 site, 1 printer and 1 cutter.

## Steps

1. Open **Subscription** (*Abonnement*) on the portal.
2. In the **Usage** section (*Utilisation*), read the **Sites**, **Machines** (printers and cutters) and **Backups** (*Sauvegardes*) lines.

   ![Subscription page with the Usage section: sites, machines and backups counted, and the table per site](../../images/cloud-abonnement.png)
3. In the table, find for each site the number of **Printers** (*Imprimantes*) and **Cutters** (*Découpeurs*), and the **Inventory received** column (*Inventaire reçu*).
4. If **Inventory received** shows "jamais (serveur RIP à mettre à jour)" (never, RIP server needs updating), update the server: it will then send its inventory.

## Good to know

- The server declares its machines (identifier, name, model) when it opens the connection, whenever the machine catalogue changes and every 6 hours.
- Printers are counted on one side, plotters and cutting tables on the other.
- A machine that prints counts as a printer, including the Roland VersaCAMM, which cuts by itself.
- The platform answers the server with the site's plan, and says whether it includes backups or whether the site exceeds its allowance.
- If the fleet exceeds the plan, see [Bring a fleet that exceeds the plan back in line](rip-cloud-parc-depasse-l-offre.md).
- Paid plans include online backup of the configuration. See [Understand online backups](../rip-cloud-sauvegardes/rip-cloud-comprendre-les-sauvegardes.md).

## See also

- [Choose a plan](rip-cloud-choisir-une-offre.md)
- [Bring a fleet that exceeds the plan back in line](rip-cloud-parc-depasse-l-offre.md)
- [Change plan, payment method or cancel](rip-cloud-changer-ou-resilier.md)
