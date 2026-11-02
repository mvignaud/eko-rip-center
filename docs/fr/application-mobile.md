# Application mobile de suivi

L'application **E-KO RIP** (iOS, Android) suit le RIP depuis un téléphone. Elle ne fait que
du suivi : pas d'envoi de fichier, pas de réglage d'impression ni de découpe. Le code est dans
`clients/mobile/` (Expo), sa construction est décrite dans
[clients/mobile/README.md](../../clients/mobile/README.md).

## Ce qu'elle fait

- **File** : les sorties de chaque machine, en cours d'abord. Sur une sortie : *Lancer* /
  *Reprendre* (sortie suspendue), *Pause* (sortie en attente), *Stop* (annule une sortie en
  attente ou suspendue), *Passer en tête*, *Relancer* (sortie finie). Une impression déjà
  partie vers la machine ne s'arrête pas depuis l'app : le serveur ne sait pas encore
  l'interrompre.
- **Machines** : état de chaque machine, sortie en cours, encres et rouleau restant, alertes
  en cours (Epson relevées par SNMP chaque minute).
- **Alertes** : les 200 derniers événements.
- **Notifications** : voir ci-dessous.

## Connexion

L'adresse du serveur (`192.168.10.109:8640`) puis, si des comptes sont déclarés, un compte
du RIP (opérateur suffit). Le nom et le mot de passe restent dans le trousseau du téléphone
pour rouvrir la session quand elle expire (12 h, ou redémarrage du serveur).

Hors de l'atelier, **Se connecter au cloud** ouvre la plate-forme E-KO RIP
(`platform.e-ko.rip`) dans le navigateur : mot de passe, Google ou Facebook, puis
autorisation de l'app. L'app affiche ensuite les sites du compte (en ligne ou non) ; un
toucher ouvre le serveur à travers le relais, sans second mot de passe, avec le rôle donné
dans le portail. Le jeton d'app (180 jours) reste dans le trousseau du téléphone ; on le
révoque par **Se déconnecter du cloud** ou dans le portail (Mon compte › Apps connectées).
Voir [cloud.md](cloud.md). Sans compte cloud, il faut un VPN.

Juste après la connexion au cloud, l'app demande l'autorisation des notifications et
inscrit le téléphone sur la plate-forme (`POST /api/moi/push`) : site hors ligne, site de
retour, nouvelle app connectée au compte. Les alertes des machines viennent, elles, du
serveur RIP (ci-dessous), qui les envoie au service push d'Expo : elles arrivent partout.
Un téléphone inscrit à travers le cloud reste destinataire tant qu'il se réinscrit (à chaque
ouverture de l'app) au moins une fois tous les 30 jours.

## Notifications

| Événement | Quand |
|---|---|
| Alerte de la machine | une alerte nouvelle de la Printer-MIB (capot ouvert, support épuisé…), hors états ordinaires (prête, impression, chauffe, démarrage, veille). Les textes anglais de la machine sont traduits en français (`ALERTES_FR` de `serveur/src/mobile.rs`) ; un texte inconnu reste tel quel |
| Encre presque vide | une encre passe à 5 % ou moins |
| Cartouche vide | une encre passe à 0 % |
| Machine injoignable | 3 relevés SNMP manqués de suite ; puis « répond de nouveau » |
| Sortie en échec | une impression ou une découpe passe en échec |
| Rendu en échec | le rendu d'un travail échoue (une fois par travail) |
| Fin d'impression | seulement si « Fins d'impression » est coché dans Réglages |

Après un redémarrage du serveur, le premier relevé et le premier état de la file ne font que
poser le point de départ : pas de rafale de notifications.

Le serveur garde les téléphones dans `<données>/mobile/appareils.json` (droits 0600) et les
événements dans `<données>/mobile/evenements.json`. Un téléphone dont le compte est
supprimé ne reçoit plus rien ; un jeton qu'Expo dit périmé est oublié.

Le serveur doit joindre `https://exp.host` en sortie. Si la « sécurité renforcée des push »
est activée sur le projet Expo, mettre son jeton d'accès dans `EKO_RIP_EXPO_JETON`
(`/etc/default/eko-rip-serveur`).

## API

Mêmes règles que le reste de l'API (signature `EKO-RIP-Client/` dans le `User-Agent`,
session).

| Route | Rôle |
|---|---|
| `GET /api/mobile/etat` | machines (dernier relevé, sortie en cours, en attente, suspendues) et numéro du dernier événement |
| `GET /api/mobile/evenements?depuis=<n>&limite=<n>` | événements plus récents que `depuis`, les plus récents d'abord (200 au plus) |
| `POST /api/mobile/appareils` | `{jeton, nom, plateforme, fins}` : inscrit le téléphone au nom du compte connecté |
| `POST /api/mobile/appareils/retirer` | `{jeton}` |
| `POST /api/mobile/essai` | `{jeton}` : notification d'essai vers un téléphone inscrit |

Les actions sur la file passent par les routes existantes (`GET /api/file`,
`POST /api/file/{id}/{action}`).
