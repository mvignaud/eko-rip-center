---
titre: Relier un serveur RIP au cloud
description: Rattachez le serveur RIP de votre atelier à la plate-forme avec un code de 10 minutes, sans ouvrir de port.
sous_rubrique: rip-cloud-premiers-pas
position: 3
mots_cles: rattacher, appairage, code, serveur, cloud
---
Relier le serveur permet de l'atteindre depuis l'extérieur de l'atelier.
Il faut un code sur le serveur, puis sa saisie sur le portail.

## Avant de commencer

- Le serveur doit avoir **au moins un compte utilisateur**. Sinon le cloud refuse de s'ouvrir.
- Le serveur doit avoir accès à Internet.
- Votre adresse e-mail de plate-forme doit être confirmée.
- Comptez deux minutes. Vous pouvez relier jusqu'à 10 sites par compte.

## Étapes

1. Ouvrez l'interface d'administration du serveur RIP, avec un compte administrateur.
2. Allez dans **Système › Cloud E-KO RIP**.
3. Cliquez sur **Connecter à la plate-forme**. Le serveur affiche un code court, valable 10 minutes.
4. Sur le portail, connectez-vous avec votre compte.
5. Allez dans **Mes sites**, rubrique **Rattacher un serveur RIP**.
6. Saisissez le code, puis cliquez sur **Rattacher**.

   ![Page Mes sites, rubrique Rattacher un serveur RIP, avec un code saisi et le bouton Rattacher encadré](../images/cloud-rattacher-code.png)

Quelques secondes plus tard, **Système › Cloud E-KO RIP** indique **Connecté : accessible à distance** et affiche l'**Adresse à distance**.

Le site apparaît dans **Mes sites**, avec son adresse à distance, son état et votre rôle.

![Page Mes sites avec le site Atelier Martin · Lyon, son adresse à distance, son état et le rôle Propriétaire](../images/cloud-mes-sites.png)

## Bon à savoir

- Le serveur affiche aussi un lien : l'ouvrir mène directement à la saisie du code.
- Le compte qui saisit le code devient propriétaire du site.
- Après une coupure de réseau, le serveur se reconnecte tout seul.
- Le rattachement se fait uniquement depuis l'atelier, jamais par le cloud.

## Voir aussi

- [Installer le serveur RIP](../rip-premiers-pas/rip-installer-le-serveur.md)
- [Accéder à son serveur à distance](rip-cloud-acceder-a-distance.md)
- [Le code a expiré ou est refusé](../rip-cloud-depannage/rip-cloud-code-expire.md)
- [Documentation du cloud](https://e-ko.rip/documentation/cloud/)
