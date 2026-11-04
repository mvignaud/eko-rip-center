---
titre: Comprendre le pont ERP
description: Le pont ERP relie le RIP à l'ERP E-KO : ce qui circule, dans quel sens, sans aucun montant et sans port à ouvrir.
sous_rubrique: rip-erp
position: 20
mots_cles: pont erp, erp, contrat, eko-rip-erp, principes, hmac, https, version du contrat
---

Le pont ERP est l'échange signé entre le RIP et l'ERP de la Suite E-KO : l'ERP envoie des commandes et des actions, le RIP renvoie le catalogue, les états des travaux et des sorties, et la consommation réelle. Cet article présente les principes avant la mise en place.

## Avant de commencer

Il s'adresse à l'administrateur du RIP et à la personne qui gère l'ERP. Pour la mise en place, voir [Créer le jeton et le secret du pont ERP](rip-creer-le-jeton-et-le-secret-du-pont-erp.md) puis [Activer le pont ERP](rip-activer-le-pont-erp.md).

## Étapes

1. Retenez le sens des échanges : le RIP **sort** vers l'ERP en HTTPS. Aucun port n'est ouvert à l'atelier. Les seules routes que l'ERP appelle sur le RIP sont celles du préfixe `/api/erp/`, quand le réseau le permet (même site, VPN).
2. Retenez qu'**aucun montant ne transite** : le RIP envoie des quantités physiques (surfaces, longueurs, encre, temps) et l'ERP valorise.
3. Repérez les quatre familles d'échanges :
   - le **catalogue** (machines, supports, modes, réglages enregistrés, profils) que le RIP publie : voir [Consulter les événements envoyés à l'ERP](rip-consulter-les-evenements-envoyes-a-l-erp.md) ;
   - les **états** et la **consommation** des travaux et des sorties : voir [Comprendre les états des travaux et des sorties](rip-comprendre-les-etats-des-travaux-et-des-sorties.md) ;
   - les **travaux** que l'ERP envoie au RIP : voir [Recevoir les travaux envoyés par l'ERP](rip-recevoir-les-travaux-envoyes-par-l-erp.md) ;
   - les **actions** et les **estimations** demandées par l'ERP : voir [Piloter la file depuis l'ERP](rip-piloter-la-file-depuis-l-erp.md) et [Estimer un dossier depuis l'ERP](rip-estimer-un-dossier-depuis-l-erp.md).
4. Retenez que chaque événement est signé et numéroté : voir [Vérifier la signature des événements du pont ERP](rip-verifier-la-signature-des-evenements-du-pont-erp.md) et [Comprendre les instances du pont ERP](rip-comprendre-les-instances-du-pont-erp.md).

## Bon à savoir

- **Version du contrat** : il s'appelle `eko-rip-erp/1`. Il est renvoyé par `GET /api/erp/contrat` et porté par chaque événement. Un ajout compatible (champ, type d'événement, état nouveau documenté) garde `/1` : l'ERP **ignore** les champs et les types qu'il ne connaît pas. Un changement incompatible passe à `/2` ; le RIP l'annonce dans son journal des versions avant de le livrer.
- Le jeton de l'ERP et le secret de signature se créent sur le serveur du RIP, par un administrateur : ils ne s'affichent qu'une fois.
- Les routes `/api/erp/` restent fermées sans jeton, même quand l'accès humain est libre (serveur sans compte).
- Les erreurs ont un corps JSON `{ "code": "<code stable>", "detail": "<texte ou null>" }`.
- Si l'écran du pont affiche **en erreur**, **remplacé** ou **anomalie**, voir [Résoudre un pont ERP en erreur, remplacé ou en anomalie](rip-resoudre-un-pont-erp-en-erreur.md).

## Voir aussi

- [Créer le jeton et le secret du pont ERP](rip-creer-le-jeton-et-le-secret-du-pont-erp.md)
- [Activer le pont ERP](rip-activer-le-pont-erp.md)
- [Comprendre les échanges du pont ERP](rip-comprendre-les-echanges-du-pont-erp.md)
- [Consulter le glossaire du RIP](../rip-premiers-pas/rip-consulter-le-glossaire.md)
