---
titre: Comprendre les messages d'erreur du cloud
description: Le sens des messages affichés par le serveur et par le portail (hors ligne, signature, lecture seule, offre dépassée) et la solution.
sous_rubrique: rip-cloud-depannage
position: 21
mots_cles: erreur, message, injoignable, lecture seule, signature, 402, 503, ferme par cloud
---

Le serveur et le portail affichent des messages courts quand le cloud ne peut pas aboutir.
Ce tableau donne le sens de chacun et la marche à suivre.

## Avant de commencer

- Repérez le message exact à l'écran, dans le client, l'application mobile ou le navigateur.
- Beaucoup de messages disparaissent seuls après une minute : le serveur se reconnecte sans intervention.

## Étapes

1. Retrouvez votre message dans le tableau.

   | Message ou réponse | Sens | Solution |
   |---|---|---|
   | **Plate-forme injoignable : vérifier la connexion Internet du serveur.** | Le serveur n'atteint pas la plate-forme. | Vérifiez Internet sur place. Voir [Résoudre un site hors ligne](rip-cloud-site-hors-ligne.md). |
   | **Coupé, nouvel essai sous une minute** | La connexion est tombée. | Attendez une minute. |
   | **Le cloud demande au moins un compte sur ce serveur…** | Aucun compte sur le serveur. | Voir [Résoudre « le cloud demande au moins un compte »](rip-cloud-cloud-demande-un-compte.md). |
   | **Ce serveur est déjà rattaché à la plate-forme.** | Vous demandez un code alors que le serveur est relié. | Détachez-le d'abord si vous voulez le relier à un autre compte. Voir [Détacher un serveur RIP du cloud](../rip-cloud-compte/rip-cloud-detacher-un-serveur.md). |
   | **Ce serveur n'est pas rattaché au cloud E-KO RIP.** | Action impossible sans rattachement. | Reliez le serveur. |
   | **Impossible à distance : à faire depuis l'atelier.** | Fonction réservée à l'atelier. | Voir [Savoir ce qui ne passe jamais par le cloud](../rip-cloud-premiers-pas/rip-cloud-ce-qui-reste-reserve-a-l-atelier.md). |
   | **Ordre du cloud refusé : signature absente, fausse ou périmée.** | Un ordre de restauration non signé ou trop ancien. | Relancez la restauration depuis le portail. |
   | **Votre rôle sur ce site (cloud) est en lecture seule.** | Votre rôle ne permet pas d'écrire. | Demandez un autre rôle au propriétaire. Voir [Comprendre les rôles sur un site](../rip-cloud-compte/rip-cloud-roles-sur-un-site.md). |
   | Réponse « site hors ligne » (503) | Le site est hors ligne ou inconnu : la réponse est la même. | Vérifiez l'adresse, puis l'état du serveur. |
   | Réponse « jeton invalide » (401) dans une app | L'accès de l'app a expiré ou été révoqué. | Reconnectez l'app avec **Se connecter au cloud**. |
   | Réponse « offre dépassée » (402), page explicative | L'accès à distance est suspendu. | Voir [Rétablir un accès à distance suspendu](rip-cloud-acces-suspendu.md). |
   | Envoi refusé : fichier trop gros | Une demande dépasse 2 Go. | Réduisez la taille de l'envoi. |

2. Appliquez la solution, puis recommencez.
3. Si le message reste, joignez le support en tenant le message exact : [Joindre le support en direct](rip-cloud-joindre-le-support.md).

## Bon à savoir

- Les messages du serveur s'affichent dans **Système › Cloud E-KO RIP**, sous **Dernière erreur**.
- Un site inconnu et un site hors ligne reçoivent volontairement la même réponse.
- Les noms entre parenthèses (401, 402, 503) sont les codes de réponse web, utiles au support.

## Voir aussi

- [Lire l'état du cloud sur le serveur](../rip-cloud-premiers-pas/rip-cloud-lire-l-etat-du-cloud-sur-le-serveur.md)
- [Résoudre un site hors ligne](rip-cloud-site-hors-ligne.md)
- [Joindre le support en direct](rip-cloud-joindre-le-support.md)
