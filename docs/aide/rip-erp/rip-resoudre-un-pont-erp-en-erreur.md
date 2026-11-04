---
titre: Résoudre un pont ERP en erreur, remplacé ou en anomalie
description: Lisez l'état du pont dans Système et corrigez-le : signature refusée, signature expirée, ERP injoignable, remplacé ou anomalie.
sous_rubrique: rip-erp
position: 23
mots_cles: pont erp, erreur, signature refusée, signature expirée, injoignable, remplacé, anomalie, seq_manquant, 429
---

Quand le pont ERP n'est pas **Actif**, l'écran **Système › Pont ERP** dit pourquoi. Cet article donne la marche à suivre pour chaque cas.

## Avant de commencer

Vous êtes administrateur du serveur. Rien n'est perdu pendant une panne : tout part au retour, dans l'ordre.

## Étapes

1. Ouvrez **Administration…**, vue **Système**, section **Pont ERP**, et lisez **État** et **Dernière erreur** (avec son code).
2. Si l'état est **en erreur**, suivez le code :
   - signature refusée : le secret du RIP est différent de celui de l'ERP ; recollez le même secret des deux côtés ;
   - signature expirée : l'horloge du serveur est décalée de plus de 5 minutes ; vérifiez-la avec `timedatectl` ;
   - ERP injoignable : vérifiez le réseau, le DNS et le certificat de l'ERP ;
   - autre erreur (401, 403, 404, 422, 429, 5xx) : le code est affiché tel quel ; voir [Comprendre les échanges du pont ERP](rip-comprendre-les-echanges-du-pont-erp.md).
3. Si l'état est **remplacé**, une autre installation parle à l'ERP à la place de celle-ci, ou la séquence a été remise à zéro depuis l'écran de l'ERP. Si c'est voulu, ne faites rien. Sinon, cliquez **Tirer une nouvelle instance** et confirmez : l'autre installation s'arrêtera à son tour.
4. Si l'état est **anomalie**, l'ERP et le RIP ne comptent plus pareil. Vérifiez avec l'ERP, puis cliquez **Tirer une nouvelle instance**.
5. Contrôlez que **Dernier contact** se met à jour et que le nombre d'événements accusés augmente.

## Bon à savoir

- En erreur, le RIP réessaie après 60 secondes, puis toutes les 5 minutes. Il ne saute jamais un événement refusé.
- Sur `429` (trop de requêtes), le RIP attend la durée donnée par l'ERP (1 heure au plus), sinon 60 secondes, avant tout nouvel appel.
- Tirer une nouvelle instance fait repartir le journal de 1 : l'ERP prend ce RIP pour une installation neuve et rejoue tout.
- Une remise à zéro de la séquence depuis l'écran de l'ERP ressemble à un remplacement : le RIP s'arrête et attend que vous tiriez une nouvelle instance.

## Voir aussi

- [Activer le pont ERP](rip-activer-le-pont-erp.md)
- [Comprendre les instances du pont ERP](rip-comprendre-les-instances-du-pont-erp.md)
- [Vérifier la signature des événements du pont ERP](rip-verifier-la-signature-des-evenements-du-pont-erp.md)
- [Surveiller le service du serveur](../rip-administration/rip-surveiller-le-service-du-serveur.md)
