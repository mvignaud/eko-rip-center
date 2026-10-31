# Politique de confidentialité — E-KO RIP (application mobile)

Dernière mise à jour : 31 octobre 2026

**Éditeur** : 2M Numérique — contact : mathieu.vignaud2@gmail.com

E-KO RIP est une application de suivi pour le serveur d'impression (RIP) E-KO de votre entreprise.
Elle ne collecte aucune donnée pour le compte de l'éditeur : il n'y a ni compte E-KO, ni publicité, ni outil de mesure d'audience, ni pistage.

## Données traitées

- **Adresse du serveur, nom d'utilisateur et mot de passe** : saisis par vous pour vous connecter à votre propre serveur RIP. Ils sont conservés uniquement sur votre téléphone, dans le stockage sécurisé du système (Keychain sur iOS, Keystore sur Android), et envoyés uniquement à ce serveur.
- **Données de production** (file d'impression, machines, niveaux d'encre, alertes) : lues depuis votre serveur et affichées dans l'application. Elles ne sont pas transmises à l'éditeur.
- **Jeton de notification (push) et nom de l'appareil** : si vous activez les notifications, l'application obtient un jeton auprès du service de notifications d'Expo, puis l'envoie à votre serveur RIP avec le nom et le système (iOS ou Android) de l'appareil, afin que le serveur puisse vous notifier. Le serveur transmet ensuite les notifications au service Expo (exp.host), qui les achemine via Apple (APNs) ou Google (FCM).
- **Préférences** (notifications actives, dernière alerte vue) : stockées sur votre téléphone.

## Ce que nous ne faisons pas

Aucune donnée n'est envoyée à l'éditeur. Aucune donnée n'est vendue ni partagée à des fins publicitaires. L'application n'utilise ni publicité, ni traceur, ni identifiant publicitaire, ni localisation, ni contacts, ni photos.

## Mode démo

Le mode démo utilise des données fictives générées dans l'application. Il n'effectue aucune connexion réseau et n'enregistre rien sur votre téléphone.

## Destinataires

- Votre serveur RIP, administré par votre entreprise : c'est lui qui décide de la conservation des données qu'il reçoit (notamment le jeton de notification).
- Expo (service de notifications push), Apple et Google, uniquement pour l'acheminement des notifications.

## Durée de conservation et suppression

Les données stockées sur le téléphone sont supprimées lorsque vous vous déconnectez (identifiants) ou désinstallez l'application. En vous déconnectant ou en coupant les notifications, l'application demande aussi à votre serveur de retirer le jeton de votre appareil.

## Vos droits

Pour toute question ou demande relative à vos données (accès, rectification, effacement), contactez votre administrateur (responsable du serveur RIP) ou l'éditeur à mathieu.vignaud2@gmail.com. Vous pouvez aussi saisir la CNIL (www.cnil.fr).

## Public

L'application s'adresse à des utilisateurs professionnels majeurs. Elle n'est pas destinée aux enfants.

## Modifications

Cette politique peut évoluer ; la date de mise à jour figure en haut de la page.
