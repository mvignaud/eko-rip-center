---
titre: Understand the cloud's error messages
description: The meaning of the messages shown by the server and the portal (offline, signature, read-only, plan exceeded) and the fix.
sous_rubrique: rip-cloud-depannage
position: 21
mots_cles: error, message, unreachable, read-only, signature, 402, 503, blocked through cloud
slug: rip-cloud-understand-the-clouds-error-messages
traduction_de: rip-cloud-comprendre-les-messages-d-erreur
---

The server and the portal show short messages when the cloud cannot go through.
This table gives the meaning of each and what to do. The messages appear in French on screen; the English meaning follows each one.

## Before you start

- Note the exact message on screen, in the client, the mobile app or the browser.
- Many messages disappear by themselves after a minute: the server reconnects without any action.

## Steps

1. Find your message in the table.

   | Message or response | Meaning | Fix |
   |---|---|---|
   | **Plate-forme injoignable : vérifier la connexion Internet du serveur.** (Platform unreachable: check the server's Internet connection.) | The server cannot reach the platform. | Check the Internet connection on site. See [Fix an offline site](rip-cloud-site-hors-ligne.md). |
   | **Coupé, nouvel essai sous une minute** (Disconnected, retrying within a minute) | The connection dropped. | Wait a minute. |
   | **Le cloud demande au moins un compte sur ce serveur…** (The cloud requires at least one account on this server…) | No account on the server. | See [Fix "the cloud requires at least one account"](rip-cloud-cloud-demande-un-compte.md). |
   | **Ce serveur est déjà rattaché à la plate-forme.** (This server is already linked to the platform.) | You asked for a code while the server is linked. | Unlink it first if you want to link it to another account. See [Unlink a RIP server from the cloud](../rip-cloud-compte/rip-cloud-detacher-un-serveur.md). |
   | **Ce serveur n'est pas rattaché au cloud E-KO RIP.** (This server is not linked to E-KO RIP Cloud.) | The action is impossible without linking. | Link the server. |
   | **Impossible à distance : à faire depuis l'atelier.** (Not possible remotely: do this from the workshop.) | A function reserved for the workshop. | See [Know what never goes through the cloud](../rip-cloud-premiers-pas/rip-cloud-ce-qui-reste-reserve-a-l-atelier.md). |
   | **Ordre du cloud refusé : signature absente, fausse ou périmée.** (Cloud order refused: signature missing, wrong or expired.) | An unsigned or too old restore order. | Start the restore again from the portal. |
   | **Votre rôle sur ce site (cloud) est en lecture seule.** (Your role on this site (cloud) is read-only.) | Your role does not allow writing. | Ask the owner for another role. See [Understand roles on a site](../rip-cloud-compte/rip-cloud-roles-sur-un-site.md). |
   | "Site offline" response (503) | The site is offline or unknown: the response is the same. | Check the address, then the server's status. |
   | "Invalid token" response (401) in an app | The app's access has expired or been revoked. | Reconnect the app with **Sign in to the cloud** (*Se connecter au cloud*). |
   | "Plan exceeded" response (402), explanatory page | Remote access is suspended. | See [Restore suspended remote access](rip-cloud-acces-suspendu.md). |
   | Upload refused: file too large | A request exceeds 2 GB. | Reduce the size of the upload. |

2. Apply the fix, then try again.
3. If the message stays, contact support with the exact message: [Contact live support](rip-cloud-joindre-le-support.md).

## Good to know

- The server's messages appear in **System › E-KO RIP Cloud**, under **Last error** (*Dernière erreur*).
- An unknown site and an offline site deliberately get the same response.
- The numbers in parentheses (401, 402, 503) are web response codes, useful to support.

## See also

- [Read the cloud status on the server](../rip-cloud-premiers-pas/rip-cloud-lire-l-etat-du-cloud-sur-le-serveur.md)
- [Fix an offline site](rip-cloud-site-hors-ligne.md)
- [Contact live support](rip-cloud-joindre-le-support.md)
