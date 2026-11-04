---
titre: Consulter la liste des e-mails de la plate-forme
description: Les e-mails que la plate-forme envoie : mot de passe, confirmation, alertes, résumé, facturation, sauvegardes.
sous_rubrique: rip-cloud-compte
position: 20
mots_cles: e-mails, alertes, résumé hebdomadaire, expéditeur, notifications par e-mail
---

La plate-forme vous écrit dans quelques situations précises.
Cet article liste chaque e-mail, son destinataire et son déclencheur.

## Avant de commencer

- Les messages viennent de **E-KO RIP Cloud** (`no-reply@e-ko.fr`). Il n'y a rien à configurer. Ne répondez pas à ces messages.
- Pour écrire à l'équipe, utilisez [le support en direct](../rip-cloud-depannage/rip-cloud-joindre-le-support.md).

## Étapes

1. Repérez dans le tableau l'e-mail que vous avez reçu, ou que vous attendez.

   | E-mail | Quand | Destinataire |
   |---|---|---|
   | Mot de passe oublié | À la demande, lien valable 1 heure, utilisable une seule fois. | Le compte |
   | Confirmation de l'adresse | À l'inscription par mot de passe, lien valable 24 heures. | Le compte |
   | Site hors ligne | Le serveur est injoignable depuis 15 minutes. | Propriétaires et administrateurs du site |
   | Site de nouveau en ligne | Le serveur répond de nouveau, seulement si l'alerte « hors ligne » est partie. | Propriétaires et administrateurs du site |
   | Nouvelle app connectée | Une app vient de se connecter au compte (appareil, date, lien vers **Mon compte**). | Le titulaire du compte |
   | Résumé hebdomadaire | Chaque lundi vers 8 h (heure de Paris), pour chaque site. | Propriétaires et administrateurs |
   | Abonnement, facture, essai, mandat SEPA, paiement refusé, retour à l'offre Gratuite | Selon la vie de l'abonnement. | Le contact de facturation |
   | Offre dépassée, accès à distance suspendu | Le parc dépasse l'offre. | Propriétaire et administrateurs des sites |
   | Espace de sauvegarde utilisé à 80 % ou plein | Le quota de l'offre se remplit. | Le propriétaire |
   | Sauvegarde en échec | Aucune sauvegarde de nuit reçue depuis plus de deux jours. | Propriétaire et administrateurs du site |
   | Sauvegarde restaurée | Une restauration vient d'être appliquée. | Propriétaire et administrateurs |

2. Pour décider ce que vous recevez, ouvrez **Mon compte** : voir [Choisir les e-mails reçus](rip-cloud-choisir-les-emails.md).
3. Si un e-mail n'arrive pas, voir [Résoudre les e-mails qui n'arrivent pas](../rip-cloud-depannage/rip-cloud-emails-non-recus.md).

## Bon à savoir

- Au plus **une alerte « hors ligne » par site et par heure**.
- Le résumé hebdomadaire indique, pour chaque site : le nombre de coupures (d'une minute au moins), la durée totale hors ligne, la plus longue coupure, les alertes envoyées, les apps connectées dans la semaine et l'état des sauvegardes. Un seul résumé part par semaine.
- Pour « Mot de passe oublié », la page répond de la même façon que l'adresse existe ou non ; 3 e-mails au plus par heure et par adresse. Aucun e-mail ne part pour un compte suspendu.
- L'e-mail de confirmation se renvoie 3 fois au plus par heure.
- Les réponses du support en direct arrivent par e-mail quand la page est fermée, quelles que soient vos cases.
- L'e-mail « nouvelle app connectée » et les e-mails de sécurité sont toujours envoyés.

## Voir aussi

- [Choisir les e-mails reçus](rip-cloud-choisir-les-emails.md)
- [Comprendre les notifications du cloud](rip-cloud-comprendre-les-notifications.md)
- [Résoudre un site hors ligne](../rip-cloud-depannage/rip-cloud-site-hors-ligne.md)
