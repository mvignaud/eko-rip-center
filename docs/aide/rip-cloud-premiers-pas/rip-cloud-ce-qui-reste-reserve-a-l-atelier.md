---
titre: Savoir ce qui ne passe jamais par le cloud
description: Le pont ERP, le renvoi de travaux entre deux RIP et le rattachement au cloud restent réservés à l'atelier.
sous_rubrique: rip-cloud-premiers-pas
position: 22
mots_cles: limites, ferme par cloud, pont ERP, renvoi, atelier, distance
---

Certaines fonctions restent réservées à l'atelier, même quand le cloud est actif.
Cet article les liste et explique le message que vous verrez à distance.

## Avant de commencer

- Cette règle protège l'atelier : elle ne se désactive pas.
- Elle s'applique à toute demande qui arrive par le cloud, quel que soit le compte utilisé.

## Étapes

1. Repérez ce qui est réservé à l'atelier :
   - le **pont avec l'ERP** et ses réglages ;
   - le **renvoi de travaux entre deux RIP** ;
   - le **rattachement** et le **détachement** du cloud lui-même.
2. Si vous en avez besoin, connectez-vous depuis l'atelier : sur le réseau local, avec l'adresse locale du serveur.
3. Si vous êtes à distance et que le message **Impossible à distance : à faire depuis l'atelier.** s'affiche, retournez à l'atelier ou demandez à une personne sur place de faire l'opération.

## Bon à savoir

- Le serveur reconnaît lui-même ce qui arrive par le cloud : une demande du réseau local ne peut pas porter cette marque.
- Le serveur refuse aussi, par prudence, toute demande venue par le cloud quand **aucun compte** n'est déclaré sur le serveur. Voir [Résoudre « le cloud demande au moins un compte »](../rip-cloud-depannage/rip-cloud-cloud-demande-un-compte.md).
- L'ordre de **restauration** d'une sauvegarde envoyé par la plate-forme fait exception : le serveur l'accepte seulement s'il est signé. Voir [Restaurer une sauvegarde de la configuration](../rip-cloud-sauvegardes/rip-cloud-restaurer-une-sauvegarde.md).

## Voir aussi

- [Comprendre les messages d'erreur du cloud](../rip-cloud-depannage/rip-cloud-comprendre-les-messages-d-erreur.md)
- [Comprendre la sécurité du cloud](rip-cloud-securite.md)
- [Accéder à son serveur RIP à distance](rip-cloud-acceder-a-distance.md)
