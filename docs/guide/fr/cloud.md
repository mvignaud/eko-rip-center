Le cloud **E-KO RIP** donne accès au serveur RIP de l'atelier depuis l'extérieur : à la maison, chez un client, en déplacement. Il fonctionne sans VPN et sans rien changer à la box Internet. Le service est gratuit pour tout serveur E-KO RIP Center.

![Schéma du cloud : atelier, connexion sortante, plate-forme, postes et téléphones à distance](../images/schema-cloud.svg "Le serveur de l'atelier se connecte à la plate-forme, les postes et téléphones à distance passent par elle")

## À quoi sert le cloud

Sans le cloud, le serveur RIP n'est joignable que sur le réseau de l'atelier. Avec le cloud :

- les postes (Mac, Linux, Windows) se connectent au serveur depuis n'importe où ;
- l'interface d'administration s'ouvre dans un navigateur, comme à l'atelier ;
- l'[application mobile](application-mobile.md) suit la production hors de l'atelier ;
- des notifications et des e-mails préviennent quand le serveur ne répond plus.

Le principe est simple : c'est le serveur de l'atelier qui ouvre lui-même une connexion vers la plate-forme. **Aucun port n'est à ouvrir** sur la box. Les postes et les téléphones à distance passent par la plate-forme pour atteindre le serveur.

> [!NOTE]
> Le cloud est facultatif. Le serveur continue de fonctionner normalement à l'atelier, avec ou sans lui.

## Créer un compte

Le portail de la plate-forme se trouve à l'adresse <https://platform.e-ko.rip>.

![Page de connexion du portail](../images/cloud-connexion.png "La page de connexion du portail")

1. Ouvrir <https://platform.e-ko.rip>.
2. Choisir **Créer un compte**.
3. Saisir le nom (ou le nom de l'atelier), l'adresse e-mail et un mot de passe d'au moins 10 caractères.
4. Accepter les conditions d'utilisation, puis valider.

Si la page propose **Google** ou **Facebook**, ces boutons permettent aussi de s'inscrire sans mot de passe.

### Confirmer l'adresse e-mail

Un e-mail de confirmation est envoyé à l'inscription par mot de passe.

1. Ouvrir l'e-mail envoyé par **E-KO RIP Cloud** (`no-reply@e-ko.fr`).
2. Cliquer sur le lien de confirmation, valable 24 heures.
3. Cliquer sur le bouton de la page pour valider.

Tant que l'adresse n'est pas confirmée, un bandeau l'indique sur **Mes sites** et **Mon compte**. Il propose **Renvoyer l'e-mail**. Rattacher un serveur est refusé jusqu'à la confirmation. Le reste du portail fonctionne.

Un compte créé avec Google est confirmé d'office.

> [!TIP]
> Si l'e-mail n'arrive pas, regarder dans les courriers indésirables, puis demander un nouvel envoi depuis le bandeau.

## Rattacher un serveur RIP

Le rattachement se fait en deux temps : un code sur le serveur, puis la saisie de ce code sur le portail. Compter deux minutes.

### Avant de commencer

Le serveur doit avoir **au moins un compte utilisateur**. Sans compte, l'accès au serveur est libre sur le réseau local, et le cloud refuse de s'ouvrir pour ne pas laisser l'atelier ouvert à tous.

Pour créer un compte, voir [Administrer le serveur](administration.md#comptes). Le serveur doit aussi avoir accès à Internet (voir [Installer le serveur](serveur.md)).

### Pas à pas

1. Ouvrir l'interface d'administration du serveur RIP, avec un compte administrateur.
2. Aller dans **Système › Cloud E-KO RIP**.
3. Cliquer sur **Connecter à la plate-forme**. Le serveur affiche un code court, de la forme `KXRT-4821`, valable **10 minutes**.
4. Sur le portail, se connecter avec son compte.
5. Aller dans **Mes sites**, rubrique **Rattacher un serveur RIP**.
6. Saisir le code, puis cliquer sur **Rattacher**.

![Page Mes sites du portail avec la zone de saisie du code](../images/cloud-appairage.png "Saisie du code dans Mes sites")

> [!TIP]
> Le serveur affiche aussi un lien : l'ouvrir mène directement à la page de saisie du code.

Quelques secondes plus tard, le serveur est relié. L'écran **Système › Cloud E-KO RIP** indique **Connecté : accessible à distance** et affiche l'**Adresse à distance**. Le compte qui a saisi le code devient propriétaire du site.

![Liste Mes sites : état et adresse à distance de chaque site](../images/cloud-sites.png "La liste Mes sites")

Le portail permet de rattacher jusqu'à 10 sites par compte.

> [!NOTE]
> Après une coupure de réseau, le serveur se reconnecte tout seul, sans intervention.

## Utiliser l'accès à distance

Chaque site a sa propre adresse :

```
https://<site>.platform.e-ko.rip
```

Elle est visible dans **Système › Cloud E-KO RIP** sur le serveur, et dans **Mes sites** sur le portail. Elle sert dans trois cas.

### Dans un client de bureau

1. Ouvrir le client (voir [Mac](clients.md#client-mac), [Linux](clients.md#client-linux) ou [Windows](clients.md#client-windows)).
2. Saisir l'adresse du site à la place de l'adresse locale du serveur.
3. Se connecter avec un compte du serveur RIP.

### Dans un navigateur

1. Ouvrir l'adresse du site.
2. Se connecter avec un compte du serveur RIP.

L'interface d'administration fonctionne comme à l'atelier.

### Dans l'application mobile

Saisir l'adresse du site dans le champ **Serveur**, ou utiliser **Se connecter au cloud** (voir ci-dessous). Détails dans [Application mobile](application-mobile.md).

> [!NOTE]
> Les comptes utilisés à distance sont ceux du serveur RIP, ceux de [Administrer le serveur](administration.md#comptes). Le compte de la plate-forme sert au portail et à **Se connecter au cloud**.

## Se connecter au cloud depuis les apps

Les apps Mac, Linux et mobile proposent un bouton **Se connecter au cloud** sur leur écran de connexion. Il évite de saisir une adresse et un second mot de passe.

1. Cliquer sur **Se connecter au cloud**. Le navigateur s'ouvre sur la plate-forme.
2. Se connecter avec son compte (mot de passe, Google ou Facebook).
3. Autoriser l'application quand le portail le demande.
4. Revenir à l'application : elle affiche la liste des sites du compte, en ligne ou non.
5. Choisir le site voulu.

Les droits dans l'application suivent le rôle du compte sur le site : un propriétaire ou un administrateur est administrateur du RIP, un opérateur est opérateur, un compte en lecture ne peut rien modifier.

L'accès d'une app reste valable environ 180 jours. À chaque nouvelle app connectée, un e-mail de sécurité est envoyé au titulaire du compte.

### Déconnecter une app

1. Se connecter au portail.
2. Ouvrir **Mon compte**.
3. Dans **Apps connectées**, cliquer sur **Déconnecter toutes les apps**.

Dans l'app elle-même, **Se déconnecter du cloud** retire l'accès de l'appareil.

> [!NOTE]
> Partager un site avec d'autres comptes (collègues, prestataires) avec un rôle opérateur ou lecture est prévu bientôt. Pour l'instant, le propriétaire du site est le compte qui l'a rattaché.

## E-mails de la plate-forme

La plate-forme écrit depuis **E-KO RIP Cloud**, adresse `no-reply@e-ko.fr`. Il n'y a rien à configurer. Ne pas répondre à ces messages.

| E-mail | Quand |
|---|---|
| Mot de passe oublié | À la demande, avec un lien valable 1 heure, utilisable une seule fois. |
| Confirmation de l'adresse | À l'inscription par mot de passe, lien valable 24 heures. |
| Site hors ligne | Le serveur est injoignable depuis 15 minutes. |
| Site de nouveau en ligne | Le serveur répond de nouveau, uniquement si l'alerte « hors ligne » est partie. |
| Nouvelle app connectée | Une app vient de se connecter au compte. |
| Résumé hebdomadaire | Chaque lundi vers 8 h, pour chaque site. |

Les alertes de site hors ligne concernent les propriétaires et les administrateurs du site. Au plus une alerte est envoyée par site et par heure.

Le résumé hebdomadaire indique, pour chaque site : le nombre de coupures, la durée totale hors ligne, la plus longue coupure, les alertes envoyées et les apps connectées dans la semaine.

### Choisir les e-mails reçus

1. Ouvrir **Mon compte** sur le portail.
2. Cocher ou décocher **Recevoir les alertes** et **Recevoir le résumé hebdomadaire**.

Les deux cases sont cochées par défaut. L'e-mail « nouvelle app connectée » est toujours envoyé : c'est une mesure de sécurité.

### Mot de passe oublié

1. Sur la page de connexion, cliquer sur **Mot de passe oublié ?**.
2. Saisir l'adresse e-mail du compte.
3. Ouvrir l'e-mail reçu et cliquer sur le lien dans l'heure.
4. Choisir un nouveau mot de passe.

Pour des raisons de sécurité, la page répond de la même façon que l'adresse existe ou non. Le nouveau mot de passe ferme toutes les sessions du compte et déconnecte ses apps.

## Notifications

Les mêmes événements que les alertes par e-mail peuvent arriver en notifications : **site hors ligne**, **site de retour en ligne** et **nouvelle app connectée**. Elles suivent la case **Recevoir les alertes**.

### Dans le navigateur

Après la première connexion au portail, la page **Mes sites** propose **Recevoir les alertes sur cet ordinateur ?**

1. Cliquer sur **Activer**.
2. Autoriser les notifications quand le navigateur le demande.

Le bouton **Plus tard** repousse le choix, et la question ne revient plus ensuite. Pour décider plus tard :

1. Ouvrir **Mon compte**.
2. Dans la carte **Notifications sur cet appareil**, cliquer sur **Activer les notifications du navigateur**.
3. Pour arrêter sur un navigateur, cliquer sur **Retirer** à côté de lui.

Les notifications arrivent même quand la page du portail est fermée, tant que le navigateur est lancé.

### Dans les apps Mac, Linux et mobile

Les apps demandent l'autorisation d'afficher des notifications juste après la première connexion au cloud. Il suffit d'accepter. Si l'autorisation a été refusée, la rétablir dans les réglages du système ou du téléphone.

## Gérer son compte

**Mon compte** regroupe :

- le nom, l'adresse e-mail et la date d'inscription ;
- les connexions liées (mot de passe, Google, Facebook) ;
- les cases d'e-mails et la carte **Notifications sur cet appareil** ;
- la liste des **Apps connectées** ;
- la suppression du compte.

### Supprimer son compte

1. Ouvrir **Mon compte**.
2. Aller à la rubrique **Supprimer mon compte**.
3. Confirmer.

> [!WARNING]
> La suppression efface le compte, ses droits et **les sites dont il est propriétaire**. Leur accès à distance s'arrête aussitôt. Le serveur RIP de l'atelier, lui, n'est pas touché : il reste utilisable sur place.

## Détacher un serveur

Détacher coupe l'accès à distance, sans toucher au serveur ni à ses données.

### Depuis le serveur

1. Ouvrir **Système › Cloud E-KO RIP**.
2. Cliquer sur **Détacher de la plate-forme**.
3. Confirmer.

L'accès à distance s'arrête aussitôt. Pour le rétablir, refaire le rattachement avec un nouveau code.

### Depuis le portail

1. Ouvrir **Mes sites**.
2. Cliquer sur **Retirer** à côté du site, puis confirmer.

Seul le propriétaire peut retirer un site.

## Ce qui ne passe jamais par le cloud

Certaines fonctions restent réservées à l'atelier, même avec le cloud :

- le pont avec l'ERP et ses réglages ;
- le renvoi de travaux entre deux RIP ;
- le rattachement et le détachement du cloud lui-même : ils se font uniquement depuis l'atelier.

Le serveur refuse ces demandes quand elles arrivent par le cloud. Il les refuse aussi, par prudence, quand aucun compte n'est déclaré sur le serveur.

## Sécurité en bref

- Aucun port n'est ouvert à l'atelier : le serveur se connecte à la plate-forme, jamais l'inverse.
- Chaque site a sa propre adresse, isolée de celle des autres sites et du portail.
- Le mot de passe d'un compte est bloqué 5 minutes après 5 échecs.
- Le portail ne transmet à l'atelier que la session du serveur RIP, jamais les informations du compte de la plate-forme.

## Questions fréquentes

### Le site est « hors ligne »

Le serveur RIP de l'atelier n'est pas relié à la plate-forme.

1. Vérifier sur place que le serveur est allumé et qu'il a accès à Internet.
2. Ouvrir **Système › Cloud E-KO RIP**. Le message **Plate-forme injoignable** indique un problème de connexion Internet.
3. Attendre une minute : après une coupure, le serveur se reconnecte seul.

Si l'alerte est activée, un e-mail « de retour en ligne » confirme la reprise.

### Le code a expiré ou est refusé

Le code n'est valable que 10 minutes. Sur le serveur, cliquer de nouveau sur **Connecter à la plate-forme** pour en obtenir un nouveau. Si le portail refuse le rattachement, vérifier que l'adresse e-mail du compte est confirmée.

### Le cloud ne s'ouvre pas : « au moins un compte »

Créer un compte sur le serveur (voir [Administrer le serveur](administration.md#comptes)), puis recommencer.

### J'ai oublié mon mot de passe

Utiliser **Mot de passe oublié ?** sur la page de connexion (voir plus haut). S'il s'agit d'un compte du serveur RIP, voir [Administrer le serveur](administration.md#comptes).

### Je ne reçois pas les e-mails

Regarder dans les courriers indésirables. Vérifier l'adresse du compte dans **Mon compte** et les cases cochées. Les e-mails viennent de `no-reply@e-ko.fr`.

## Informations légales

- Conditions générales d'utilisation : <https://platform.e-ko.rip/cgu>
- Politique de confidentialité : <https://platform.e-ko.rip/confidentialite>
- Mentions légales : <https://platform.e-ko.rip/mentions-legales>

Une question ? Écrire à contact@e-ko.rip.
