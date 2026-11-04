---
titre: Créer le jeton et le secret du pont ERP
description: Créez sur le serveur le jeton que l'ERP présente au RIP et le secret qui signe les événements, puis contrôlez ce qui est déclaré.
sous_rubrique: rip-erp
position: 21
mots_cles: jeton, secret, hmac, erp jeton, erp secret, erp etat, bearer, authorization, 401, 403
---

Le pont ERP repose sur deux secrets créés sur le serveur du RIP : le **jeton**, que l'ERP présente à chaque appel, et le **secret** partagé, qui signe les événements du RIP vers l'ERP. Chacun ne s'affiche qu'une fois.

## Avant de commencer

Vous êtes administrateur du serveur, avec un terminal et un accès `sudo`. Préparez un endroit sûr pour copier les valeurs, ou l'écran de l'ERP où les coller.

## Étapes

1. Créez le jeton de l'ERP :
   `sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur erp jeton`
2. Copiez-le tout de suite et collez-le dans l'ERP : il s'affiche **une seule fois**. Le serveur n'en garde que l'empreinte SHA-256 (dans `erp.json`, droits 0600). En créer un nouveau invalide l'ancien.
3. Créez le secret de signature :
   `sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur erp secret`
4. Copiez-le aussi : il fait 64 caractères et ne s'affichera plus. Vous le collerez dans l'écran **Système › Pont ERP** (voir [Activer le pont ERP](rip-activer-le-pont-erp.md)) et dans l'ERP.
5. Contrôlez ce qui est déclaré, sans rien révéler : `… erp etat` (avec le même début de commande).
6. Dans l'ERP, configurez l'envoi du jeton dans chaque appel : en-tête `Authorization: Bearer <jeton>`.

## Bon à savoir

- Le jeton n'ouvre que les routes `/api/erp/` :

  | Cas | Réponse |
  |---|---|
  | Route `/api/erp/…` sans jeton ou jeton faux | `401` `jeton_erp_requis` |
  | Route `/api/erp/…` avec le bon jeton | traitée |
  | Jeton présenté sur une autre route | `403` `jeton_erp_hors_pont` |
- Les routes `/api/erp/` restent fermées sans jeton même quand l'accès humain est libre.
- Le secret n'est **jamais** réaffiché, ni renvoyé par l'API, ni écrit dans la sauvegarde de la configuration (qui reprend l'URL, l'activation et l'instance). Sur un serveur remonté depuis une sauvegarde, recréez-le ou collez-le de nouveau.
- Si vous avez perdu le jeton ou le secret, créez-en un nouveau et mettez l'ERP à jour.

## Voir aussi

- [Comprendre le pont ERP](rip-comprendre-le-pont-erp.md)
- [Activer le pont ERP](rip-activer-le-pont-erp.md)
- [Vérifier la signature des événements du pont ERP](rip-verifier-la-signature-des-evenements-du-pont-erp.md)
- [Remonter un serveur à partir d'une sauvegarde](../rip-administration/rip-remonter-un-serveur.md)
