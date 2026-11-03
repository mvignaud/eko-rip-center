# Politique de confidentialité — E-KO RIP (application mobile)

Dernière mise à jour : 3 novembre 2026

**Éditeur** : SAS GOLDEN PRINT — contact : contact@e-ko.rip

E-KO RIP est une application de suivi pour le serveur d'impression (RIP) E-KO de votre entreprise.
Elle se connecte directement à ce serveur ou, si vous le choisissez, à travers la plate-forme E-KO RIP Cloud. Elle ne contient ni publicité, ni outil de mesure d'audience, ni pistage.

## Données traitées

- **Adresse du serveur, nom d'utilisateur et mot de passe** : saisis par vous pour vous connecter à votre propre serveur RIP. Ils sont conservés uniquement sur votre téléphone, dans le stockage sécurisé du système (Keychain sur iOS, Keystore sur Android), et envoyés uniquement à ce serveur.
- **Données de production** (file d'impression, machines, niveaux d'encre, alertes) : lues depuis votre serveur et affichées dans l'application. Elles ne sont pas transmises à l'éditeur.
- **Jeton de notification (push) et nom de l'appareil** : si vous activez les notifications, l'application obtient un jeton auprès du service de notifications d'Expo, puis l'envoie à votre serveur RIP avec le nom et le système (iOS ou Android) de l'appareil, afin que le serveur puisse vous notifier. Le serveur transmet ensuite les notifications au service Expo (exp.host), qui les achemine via Apple (APNs) ou Google (FCM).
- **Préférences** (notifications actives, dernière alerte vue) : stockées sur votre téléphone.

## Connexion par E-KO RIP Cloud

Si vous choisissez de vous connecter avec votre compte E-KO RIP Cloud, vous vous identifiez dans une page de la plate-forme platform.e-ko.rip, éditée par GOLDEN PRINT. L'application reçoit un jeton d'accès, conservé dans le stockage sécurisé du téléphone, et accède à vos serveurs RIP à travers la plate-forme, qui relaie les échanges en flux chiffré sans les conserver.

La plate-forme enregistre alors le nom de l'appareil et, si vous activez les notifications, son jeton de notification et son système (iOS ou Android), pour vous prévenir des événements de vos sites (site hors ligne, de retour en ligne…). Ces données sont traitées par GOLDEN PRINT selon la politique de confidentialité d'E-KO RIP Cloud : https://platform.e-ko.rip/confidentialite.

## Ce que nous ne faisons pas

Sans connexion par E-KO RIP Cloud, aucune donnée n'est envoyée à l'éditeur. Aucune donnée n'est vendue ni partagée à des fins publicitaires. L'application n'utilise ni publicité, ni traceur, ni identifiant publicitaire, ni localisation, ni contacts, ni photos.

## Mode démo

Le mode démo utilise des données fictives générées dans l'application. Il n'effectue aucune connexion réseau et n'enregistre rien sur votre téléphone.

## Destinataires

- Votre serveur RIP, administré par votre entreprise : c'est lui qui décide de la conservation des données qu'il reçoit (notamment le jeton de notification).
- La plate-forme E-KO RIP Cloud (GOLDEN PRINT), uniquement si vous vous y connectez, ainsi que Cloudflare, Inc. (États-Unis), par le réseau duquel passent les échanges avec la plate-forme (voir sa [politique de confidentialité](https://platform.e-ko.rip/confidentialite)).
- Expo (650 Industries, Inc., États-Unis), Apple et Google, uniquement pour l'acheminement des notifications : ils reçoivent le jeton de l'appareil et le texte de la notification. Ces prestataires pouvant traiter les données hors de l'Union européenne, vous pouvez l'éviter en laissant les notifications désactivées.

## Durée de conservation et suppression

Les données stockées sur le téléphone sont supprimées lorsque vous vous déconnectez (identifiants) ou désinstallez l'application. En vous déconnectant ou en coupant les notifications, l'application demande aussi à votre serveur, ou à la plate-forme E-KO RIP Cloud, de retirer le jeton de votre appareil.

## Vos droits

Pour toute question ou demande relative à vos données (accès, rectification, effacement), contactez votre administrateur (responsable du serveur RIP) ou l'éditeur à contact@e-ko.rip. Vous pouvez aussi saisir la CNIL (www.cnil.fr).

## Public

L'application s'adresse à des utilisateurs professionnels majeurs. Elle n'est pas destinée aux enfants.

## Modifications

Cette politique peut évoluer ; la date de mise à jour figure en haut de la page.
