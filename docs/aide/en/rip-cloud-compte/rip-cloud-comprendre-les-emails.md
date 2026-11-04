---
titre: View the list of platform e-mails
description: The e-mails the platform sends: password, confirmation, alerts, summary, billing, backups.
sous_rubrique: rip-cloud-compte
position: 20
mots_cles: e-mails, alerts, weekly summary, sender, e-mail notifications
slug: rip-cloud-view-the-list-of-platform-emails
traduction_de: rip-cloud-comprendre-les-emails
---

The platform writes to you in a few specific situations.
This article lists each e-mail, who receives it and what triggers it.

## Before you start

- Messages come from **E-KO RIP Cloud** (`no-reply@e-ko.fr`). There is nothing to set up. Do not reply to these messages.
- To write to the team, use [live support](../rip-cloud-depannage/rip-cloud-joindre-le-support.md).

## Steps

1. Find in the table the e-mail you received, or are expecting.

   | E-mail | When | Recipient |
   |---|---|---|
   | Forgotten password | On request, link valid for 1 hour, usable once only. | The account |
   | Address confirmation | When signing up with a password, link valid for 24 hours. | The account |
   | Site offline | The server has been unreachable for 15 minutes. | The site's owners and administrators |
   | Site back online | The server answers again, only if the "offline" alert was sent. | The site's owners and administrators |
   | New app connected | An app has just connected to the account (device, date, link to **My account**). | The account holder |
   | Weekly summary | Every Monday around 8 a.m. (Paris time), for each site. | Owners and administrators |
   | Subscription, invoice, trial, SEPA mandate, payment declined, return to the Free plan | Depending on the subscription's life. | The billing contact |
   | Plan exceeded, remote access suspended | The fleet exceeds the plan. | Owner and administrators of the sites |
   | Backup space 80% used or full | The plan's quota is filling up. | The owner |
   | Backup failed | No nightly backup received for more than two days. | Owner and administrators of the site |
   | Backup restored | A restore has just been applied. | Owner and administrators |

2. To decide what you receive, open **My account** (*Mon compte*): see [Choose which e-mails you receive](rip-cloud-choisir-les-emails.md).
3. If an e-mail does not arrive, see [Fix e-mails that do not arrive](../rip-cloud-depannage/rip-cloud-emails-non-recus.md).

## Good to know

- At most **one "offline" alert per site per hour**.
- The weekly summary gives, for each site: the number of outages (of at least one minute), the total time offline, the longest outage, the alerts sent, the apps connected during the week and the backup status. Only one summary is sent per week.
- For "Forgotten password", the page answers the same way whether the address exists or not; at most 3 e-mails per hour per address. No e-mail is sent for a suspended account.
- The confirmation e-mail can be resent at most 3 times per hour.
- Live support replies arrive by e-mail when the page is closed, whatever your boxes say.
- The "new app connected" e-mail and security e-mails are always sent.

## See also

- [Choose which e-mails you receive](rip-cloud-choisir-les-emails.md)
- [Understand cloud notifications](rip-cloud-comprendre-les-notifications.md)
- [Fix an offline site](../rip-cloud-depannage/rip-cloud-site-hors-ligne.md)
