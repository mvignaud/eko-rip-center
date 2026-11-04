---
titre: Understand cloud security
description: What protects your workshop: no open port, one isolated address per site, password lockout and separate sessions.
sous_rubrique: rip-cloud-premiers-pas
position: 23
mots_cles: security, cookies, site key, password, isolation, privacy
slug: rip-cloud-understand-cloud-security
traduction_de: rip-cloud-securite
---

The cloud is designed never to open the workshop to strangers.
This article summarises the protections, with nothing for you to set up.

## Before you start

- There is nothing to configure: these protections are active as soon as the server is linked.
- For the full terms of use, see [View the terms and legal notices](rip-cloud-consulter-les-informations-legales.md).

## Steps

1. **No open port.** The server connects to the platform, never the other way round. You change nothing on your router.
2. **One key per site.** When linking, the server receives a secret key that it keeps in a protected file; it is never displayed again. The platform keeps only a fingerprint of that key. A wrong key and an unknown site get the same response.
3. **One isolated address per site.** Each site has its own address (`https://<site>.platform.e-ko.rip`). The browser keeps its cookies and pages separate from those of other sites and of the portal. The portal does not exist at a site's address.
4. **Only the RIP server's session goes to the workshop.** The portal passes on to the workshop neither your platform account's information nor its cookies.
5. **Protected passwords.** After 5 failed sign-ins, an account's password is blocked for 5 minutes. The forms only accept requests coming from the platform itself.
6. **Limited app access.** An app's access lasts about 180 days and can be revoked at any time. Connecting a new app always triggers a security e-mail.

## Good to know

- Online backups are encrypted and hosted in France.
- If you lose a device, use **Disconnect all apps** (*Déconnecter toutes les apps*) in **My account** (*Mon compte*). See [Disconnect apps from the cloud](../rip-cloud-compte/rip-cloud-deconnecter-les-apps.md).
- A new password closes all sessions and disconnects the apps.
- An offline site and an unknown site get the same response: the cloud does not reveal which sites exist.
- Uploading a file larger than 2 GB through the cloud is refused.

## See also

- [Understand E-KO RIP Cloud](rip-cloud-comprendre-le-cloud.md)
- [Know what never goes through the cloud](rip-cloud-ce-qui-reste-reserve-a-l-atelier.md)
- [Disconnect apps from the cloud](../rip-cloud-compte/rip-cloud-deconnecter-les-apps.md)
