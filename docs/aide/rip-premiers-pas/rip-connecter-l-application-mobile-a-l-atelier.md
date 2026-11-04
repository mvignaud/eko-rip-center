---
titre: Connecter l'application mobile au serveur de l'atelier
description: Saisissez l'adresse du serveur RIP et votre compte dans l'application E-KO RIP, depuis le Wi-Fi de l'atelier.
sous_rubrique: rip-premiers-pas
position: 73
mots_cles: application mobile, connexion, adresse, serveur, wifi, compte opérateur, session
---

Cet article explique comment relier l'application E-KO RIP au serveur quand vous êtes dans l'atelier. Hors de l'atelier, passez par le cloud.

## Avant de commencer

L'application est installée et le téléphone est sur le réseau Wi-Fi de l'atelier. Vous connaissez l'adresse du serveur, la même que dans les clients de bureau. Si le serveur a des comptes, munissez-vous d'un nom d'utilisateur et d'un mot de passe.

## Étapes

1. Ouvrez l'application.
2. Dans le champ **Serveur**, saisissez l'adresse du serveur, sans espace : `http://<adresse-du-serveur>:8640`.
3. Touchez **Continuer**.
4. Si le serveur demande un compte, saisissez le nom d'utilisateur et le mot de passe, puis touchez **Se connecter**.
5. Autorisez les notifications quand le téléphone le demande.

## Bon à savoir

- Un compte **Opérateur** suffit pour suivre la file et agir sur les sorties. Les comptes se gèrent dans l'administration du serveur (voir [Créer les comptes du serveur](../rip-administration/rip-creer-les-comptes.md)).
- Le nom et le mot de passe sont gardés dans l'espace sécurisé du téléphone. L'application les réutilise pour rouvrir la session quand elle expire (après 12 heures ou après un redémarrage du serveur).
- L'adresse locale n'est joignable que depuis l'atelier. Ailleurs, voir [Se connecter au serveur depuis l'extérieur avec l'application mobile](rip-connecter-l-application-mobile-via-le-cloud.md).
- Le rôle (**Administrateur** ou **Opérateur**) s'affiche dans l'onglet **Réglages**.
- L'application n'arrive pas à joindre le serveur : voir [L'application mobile n'arrive pas à joindre le serveur](../rip-depannage/rip-application-mobile-sans-serveur.md).

## Voir aussi

- [Installer l'application mobile E-KO RIP](rip-installer-l-application-mobile.md)
- [Suivre la production depuis l'application mobile](../rip-travaux/rip-suivre-la-production-sur-mobile.md)
- [Se connecter au serveur depuis un poste](rip-se-connecter-au-serveur.md)
