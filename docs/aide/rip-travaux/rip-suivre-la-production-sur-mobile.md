---
titre: Suivre la production depuis l'application mobile
description: Connectez l'application E-KO RIP à votre serveur, suivez la file, l'encre et les alertes, et activez les notifications.
sous_rubrique: rip-travaux
position: 7
mots_cles: application mobile, iphone, android, notifications, alertes, file, machines
---

L'application E-KO RIP sert au suivi : file, machines, niveaux d'encre et alertes. Elle n'envoie aucun fichier et ne règle ni l'impression ni la découpe.

## Avant de commencer

Le téléphone est sur le Wi-Fi de l'atelier. Vous avez l'adresse du serveur et, si le serveur a des comptes, un nom et un mot de passe (un compte **Opérateur** suffit). Aucun compte E-KO n'est nécessaire pour installer l'application.

## Étapes

1. Installez l'application depuis la page https://e-ko.rip/app/ puis ouvrez-la.
2. Dans le champ **Serveur**, saisissez l'adresse du serveur : `http://<adresse-du-serveur>:8640`.
3. Touchez **Continuer**.
4. Si le serveur demande un compte, saisissez le nom et le mot de passe, puis touchez **Se connecter**.
5. Ouvrez l'onglet **File** pour voir les sorties de chaque machine, les sorties en cours en premier.
6. Ouvrez l'onglet **Machines** pour l'état, les encres et le support restant.
7. Ouvrez l'onglet **Alertes** pour les 200 derniers événements.
8. Dans **Réglages**, vérifiez que **Notifications** est activé, puis touchez **Envoyer une notification d'essai**.

![Onglet File de l'application, avec les sorties en cours et en attente de chaque machine](../images/mobile-file.png)

![Onglet Machines de l'application, avec l'état, les encres et le rouleau de chaque machine](../images/mobile-machines.png)

## Bon à savoir

- Pour agir sur une sortie, touchez-la puis choisissez l'action : **Pause**, **Passer en tête**, **Stop**, **Lancer** ou **Reprendre**, **Relancer**.
- **Pause** et **Stop** n'agissent que sur une sortie qui n'est pas encore partie vers la machine. Une impression déjà envoyée ne s'arrête pas depuis l'application.
- Dans **Réglages**, **Erreurs machine** prévient des alertes, de l'encre presque vide, des échecs et des machines injoignables. **Fins d'impression** est désactivé par défaut.
- Hors de l'atelier, touchez **Se connecter au cloud** (voir la page Cloud).
- Sans notification : vérifiez l'autorisation dans les réglages du téléphone, puis l'interrupteur **Notifications**. Se déconnecter puis se reconnecter renouvelle l'inscription.
- Pour découvrir l'application sans serveur, touchez **Essayer le mode démo**.

## Voir aussi

- [Gérer la file d'une machine](rip-gerer-la-file-d-une-machine.md)
- [Relier le serveur au Cloud](../rip-cloud-premiers-pas/rip-cloud-relier-un-serveur.md)
- [Se connecter au serveur depuis un poste](../rip-premiers-pas/rip-se-connecter-au-serveur.md)
- Page détaillée : https://e-ko.rip/documentation/application-mobile/
