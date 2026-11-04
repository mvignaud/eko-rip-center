---
titre: Create the ERP bridge token and secret
description: Create on the server the token the ERP presents to the RIP and the secret that signs the events, then check what is declared.
sous_rubrique: rip-erp
position: 21
mots_cles: token, secret, hmac, erp token, erp secret, erp status, bearer, authorization, 401, 403
slug: rip-create-erp-bridge-token-and-secret
traduction_de: rip-creer-le-jeton-et-le-secret-du-pont-erp
---

The ERP bridge relies on two secrets created on the RIP server: the **token**, which the ERP presents on every call, and the shared **secret**, which signs the events from the RIP to the ERP. Each is shown only once.

## Before you start

You are the server administrator, with a terminal and `sudo` access. Prepare a safe place to copy the values, or the ERP screen where you will paste them.

## Steps

1. Create the ERP token:
   `sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur erp jeton`
2. Copy it straight away and paste it into the ERP: it is shown **only once**. The server keeps only its SHA-256 fingerprint (in `erp.json`, permissions 0600). Creating a new one invalidates the old one.
3. Create the signing secret:
   `sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur erp secret`
4. Copy it too: it is 64 characters long and will not be shown again. You will paste it into the **System › ERP bridge** screen (*Système › Pont ERP*) (see [Turn on the ERP bridge](rip-activer-le-pont-erp.md)) and into the ERP.
5. Check what is declared, without revealing anything: `… erp etat` (with the same beginning of the command).
6. In the ERP, set up sending the token on every call: header `Authorization: Bearer <token>`.

## Good to know

- The token opens only the `/api/erp/` routes:

  | Case | Response |
  |---|---|
  | `/api/erp/…` route with no token or a wrong token | `401` `jeton_erp_requis` |
  | `/api/erp/…` route with the right token | handled |
  | Token presented on another route | `403` `jeton_erp_hors_pont` |
- The `/api/erp/` routes stay closed without a token even when human access is open.
- The secret is **never** shown again, nor returned by the API, nor written in the configuration backup (which includes the URL, the activation and the instance). On a server rebuilt from a backup, create it again or paste it again.
- If you have lost the token or the secret, create a new one and update the ERP.

## See also

- [Understand the ERP bridge](rip-comprendre-le-pont-erp.md)
- [Turn on the ERP bridge](rip-activer-le-pont-erp.md)
- [Verify the signature of ERP bridge events](rip-verifier-la-signature-des-evenements-du-pont-erp.md)
- [Rebuild a server from a backup](../rip-administration/rip-remonter-un-serveur.md)
