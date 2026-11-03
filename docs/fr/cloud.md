# Cloud E-KO RIP (accès à distance)

La plate-forme **E-KO RIP Cloud** (`https://platform.e-ko.rip`) donne accès à un serveur RIP
d'atelier depuis l'extérieur, **sans VPN et sans ouvrir de port** : c'est le serveur RIP qui
ouvre lui-même une connexion sortante (HTTPS, port 443) vers la plate-forme, et la plate-forme
fait passer par elle les requêtes des postes et de l'app mobile.

Ouvert à tout serveur E-KO RIP Center : l'offre **Gratuite** (1 site, 1 imprimante, 1 découpeur)
est sans limite de durée ; les offres payantes ajoutent des machines, des sites et un historique
de sauvegardes (voir « Offres et abonnement »). État : **lots 1 à 4** (tunnel,
appairage, portail, « Se connecter au cloud » depuis les apps, une adresse par site). À
distance, on se connecte soit avec **son compte de la plate-forme** (les apps), soit avec
**un compte du serveur RIP** (navigateur, anciennes versions).

```
Poste ou téléphone hors atelier
   │ HTTPS  <site>.platform.e-ko.rip/…
   ▼
Caddy ──► eko-rip-cloud ──► tunnel du site
                ▲
                │ connexion sortante (443), HTTP/2 dedans
                │
       Serveur RIP de l'atelier (derrière la box, aucun port ouvert)
```

## Rattacher un serveur RIP

1. Le serveur doit avoir **au moins un compte** (`sudo eko-rip-serveur utilisateur ajouter <nom>`).
   Sans compte, l'accès est libre sur le réseau local : le cloud refuse alors de s'ouvrir.
2. Dans **Système › Cloud E-KO RIP**, un administrateur clique **Connecter à la plate-forme**.
   Le serveur affiche un code court (`KXRT-4821`), valable 10 minutes.
3. Sur le portail, une fois connecté : **Mes sites › Rattacher un serveur RIP**, saisir le code
   (le lien affiché par le serveur y mène directement). Le compte devient propriétaire du site.
   En secours, sur le VPS : `sudo -u eko-rip-cloud EKO_CLOUD_DATA=/var/lib/eko-rip-cloud eko-rip-cloud valider KXRT-4821`.
4. Quelques secondes plus tard, le serveur reçoit sa clé, l'enregistre dans `cloud.json`
   (droits 0600, jamais réaffichée) et ouvre le tunnel. L'écran affiche l'adresse à distance :
   `https://<site>.platform.e-ko.rip` (aussi affichée dans **Mes sites** du portail).

**Détacher de la plate-forme** coupe le tunnel aussitôt et oublie la clé ; il faudra un nouvel
appairage pour rétablir l'accès. Après une coupure réseau, le serveur rouvre le tunnel tout
seul (attente de 1 s, 2 s… jusqu'à 60 s).

Réglages (`/etc/default/eko-rip-serveur`) : `EKO_RIP_CLOUD_URL` (autre plate-forme, essais),
`EKO_RIP_CLOUD_NOM` (nom du site proposé, sinon le nom de la machine).

## Le portail

- **Inscription** libre (e-mail + mot de passe de 10 caractères au moins, CGU acceptées), ou
  **Google** / **Facebook** une fois leurs clés saisies en administration. Un e-mail déjà inscrit
  n'est lié à Google que si Google l'a vérifié ; jamais à Facebook sans connexion préalable
  (liaison depuis « Mon compte »).
- **Mes sites** : état en ligne / hors ligne, adresse à distance, rôle, lien **Sauvegardes** ;
  rattacher (autant de sites que l'offre en comprend), retirer (le propriétaire ; tunnel coupé
  aussitôt).
- **Abonnement** : offre en cours, machines comptées, moyen de paiement, factures (PDF), changer
  d'offre ou résilier. Voir « Offres et abonnement ».
- **Mon compte** : connexions liées, suppression du compte (et des sites possédés).
- **Administration** (super-administrateur, donné par `eko-rip-cloud admin <e-mail>` sur le VPS) :
  clés Google et Facebook (les secrets ne sont jamais réaffichés), avec les URL à déclarer chez
  eux (redirection, confidentialité, CGU, suppression des données) ; liste des sites (retirer) et
  des comptes (suspendre : ses sessions sont fermées).
- **E-mails** : voir la section « E-mails de la plate-forme » ci-dessous.
- **Pages légales** : `/mentions-legales`, `/cgu`, `/cgv`, `/confidentialite` (éditeur SAS GOLDEN PRINT ;
  RCS Auch 939 018 024, contact contact@e-ko.rip, pas de téléphone).
- Protections : formulaires acceptés seulement depuis la plate-forme (`Origin`) avec un jeton par
  session ; 5 échecs de mot de passe bloquent le compte 5 minutes ; limites par adresse
  (inscriptions, connexions) ; cookie de session `HttpOnly`, `SameSite=Lax`, jamais transmis
  aux ateliers ; pages servies avec une politique de sécurité stricte (CSP, pas d'iframe).
- Données : `/var/lib/eko-rip-cloud/portail.sqlite` (0600) à côté de `sites.json`.

## Utiliser l'accès à distance

Chaque site a **sa propre adresse**, `https://<site>.platform.e-ko.rip`. On la saisit telle
quelle comme adresse du serveur dans un client (Mac, Linux, Windows, mobile) ou on l'ouvre dans
un navigateur : l'interface d'administration du serveur RIP y fonctionne comme à l'atelier. La
connexion se fait avec un compte du serveur RIP.

L'ancienne forme `https://platform.e-ko.rip/s/<site>` reste servie pour les appels d'API, mais
une page y est isolée (`sandbox`, pas de script) : préférer l'adresse propre au site.

## Se connecter au cloud depuis les apps

Sur l'écran de connexion, une app propose **Serveur local** ou **Se connecter au cloud**. Le
second ouvre le navigateur sur la plate-forme (mot de passe, Google ou Facebook), demande
d'autoriser l'app, puis revient à l'app, qui affiche les sites du compte : un clic ouvre le
serveur RIP, **sans second mot de passe**. Les droits viennent du rôle sur le site, donné dans
le portail : propriétaire ou administrateur → administrateur du RIP ; opérateur → opérateur ;
lecture → opérateur en lecture seule (`lecture_seule` pour toute écriture).

Protocole (OAuth 2, code d'autorisation + PKCE, clients publics, RFC 6749, 7636, 8252) :

1. `GET /oauth/autoriser?client_id=eko-rip-center&response_type=code&redirect_uri=…&code_challenge=…&code_challenge_method=S256&state=…&appareil=<nom de l'appareil>`
   (navigateur). Adresses de retour acceptées : `http://127.0.0.1:<port>/…`,
   `http://[::1]:<port>/…`, `http://localhost:<port>/…`, `eko-rip://…`, `ekorip://…` (mobile).
2. Retour : `<redirect_uri>?code=…&state=…` (ou `error=access_denied`). Code valable 2 minutes,
   une seule fois.
3. `POST /oauth/jeton` (formulaire) : `grant_type=authorization_code`, `client_id`, `code`,
   `redirect_uri`, `code_verifier` → `{ "access_token": "ekoa_…", "token_type": "Bearer",
   "expires_in": 15552000 }` (180 jours). Erreurs : `invalid_grant`, `invalid_client`…
4. `GET /api/moi` (`Authorization: Bearer ekoa_…`) → `{ "compte": { "email", "nom" },
   "sites": [{ "id", "nom", "adresse", "en_ligne", "role" }] }`.
5. Requêtes vers `adresse` (ou `/s/<site>/…`) avec le même `Authorization`. Le relais vérifie
   le droit du compte sur ce site (sinon `503 site_hors_ligne`, comme un site absent ; jeton
   inconnu : `401 jeton_invalide`), retire le jeton et ajoute `X-EKO-Cloud-Identite` (base64url
   de `{ "compte", "email", "nom", "role": "admin" | "operateur" | "lecture" }`). Le serveur
   RIP ne croit cet en-tête que venu par son tunnel ; `GET /api/session` rend alors
   `{ "utilisateur": <e-mail>, "role", "cloud": { "email", "nom", "lecture_seule" } }`.
6. `POST /api/moi/deconnexion` (Bearer) révoque le jeton. Dans le portail, **Mon compte ›
   Apps connectées** liste les apps et les déconnecte toutes.

## E-mails de la plate-forme

Expéditeur : « E-KO RIP Cloud » `<no-reply@e-ko.fr>`. Envoi par SMTP (chiffré : STARTTLS ou TLS
direct, jamais en clair) avec les identifiants du serveur SMTP de l'ERP.

**Réglages** : Administration › carte **E-mails** : hôte, port (587 en STARTTLS, 465 en TLS
direct), sécurité, identifiant, mot de passe (jamais réaffiché ; vide = inchangé), adresse de
l'expéditeur (`no-reply@e-ko.fr` par défaut) et bouton **Envoyer un e-mail d'essai** (à
l'administrateur connecté, avec le résultat de l'envoi). Variables d'environnement facultatives
(dans `/etc/default/eko-rip-cloud`), qui priment sur les réglages : `EKO_CLOUD_SMTP_HOTE`,
`EKO_CLOUD_SMTP_PORT`, `EKO_CLOUD_SMTP_SECURITE` (`starttls` ou `tls`),
`EKO_CLOUD_SMTP_IDENTIFIANT`, `EKO_CLOUD_SMTP_MOT_DE_PASSE`, `EKO_CLOUD_SMTP_EXPEDITEUR`.
Sans hôte SMTP, les envois sont ignorés (une ligne au journal) et rien ne bloque : « Mot de
passe oublié » affiche sa réponse habituelle, et rattacher un serveur n'exige pas la
confirmation de l'adresse.

**Les envois** (hors de la requête : une page n'attend jamais le serveur SMTP ; un échec
s'écrit au journal, sans secret) :
- **Mot de passe oublié** (`/mot-de-passe/oublie`) : lien `/mot-de-passe/nouveau?jeton=…`
  valable 1 heure, à usage unique ; la réponse est la même que le compte existe ou non (3 e-mails
  par heure et par adresse ; compte suspendu : rien n'est envoyé). Le nouveau mot de passe ferme
  toutes les sessions du compte et déconnecte ses apps.
- **Confirmation de l'adresse** à l'inscription par mot de passe : lien `/email/confirmer?jeton=…`
  (24 heures, usage unique ; le clic sur la page confirme, la simple ouverture ne consomme rien).
  Tant qu'elle n'est pas confirmée, un bandeau sur « Mes sites » et « Mon compte » propose de
  renvoyer l'e-mail (3 par heure) et rattacher un serveur est refusé ; le reste fonctionne.
  Les comptes existants et ceux créés par Google (e-mail vérifié) sont confirmés d'office.
- **Site hors ligne** : le tunnel est fermé depuis 15 minutes → e-mail aux propriétaires et
  administrateurs du site, puis « de retour en ligne » (seulement si l'alerte est partie). Au plus
  une alerte par site et par heure.
- **Nouvelle app connectée** : à chaque connexion d'une app (« Se connecter au cloud »),
  e-mail de sécurité au titulaire (appareil, date, lien vers « Mon compte »). Toujours envoyé.
- **Résumé hebdomadaire** : chaque lundi vers 8 h (heure de Paris, heure d'été comprise), un
  e-mail par propriétaire ou administrateur : pour chacun de ses sites, nombre de coupures (d'une
  minute au moins), durée hors ligne, plus longue coupure, alertes envoyées, et les apps connectées
  dans la semaine. Un seul envoi par semaine (mémorisé, y compris après un redémarrage) ; le
  premier démarrage mémorise la semaine sans rien envoyer.

Dans « Mon compte », les cases **Recevoir les alertes** et **Recevoir le résumé hebdomadaire**
(activées par défaut) règlent les deux derniers envois. Journal des coupures : table
`evenements_sites` de `portail.sqlite`, purgée au-delà de 90 jours.

**Aperçu des e-mails** : `eko-rip-cloud apercu-courriels <dossier>` écrit chaque message avec des
données d'exemple (`.html` autonome et `.txt`).

## Notifications

En plus des e-mails, la plate-forme notifie les mêmes événements sur trois canaux : **site hors
ligne** (après 15 minutes), **site de retour en ligne** (si l'alerte est partie) et **nouvelle app
connectée**. Ils suivent la case **Recevoir les alertes** de « Mon compte » (propriétaires et
administrateurs du site), sauf la nouvelle app, toujours notifiée (sécurité). Un seul point
d'émission (`notifications::notifier`) fait l'e-mail, l'entrée du journal et les pushs. Les envois
réseau partent hors de la requête ; échec = une ligne au journal, sans secret. Le journal et les
pushs partent même sans SMTP réglé.

**Première connexion** : sur chaque poste, les apps demandent l'autorisation d'afficher des
notifications juste après la première connexion au cloud (Mac, Linux, Windows, mobile). Sur le
portail, après la première connexion (mot de passe, Google ou Facebook, et juste après
l'inscription), « Mes sites » propose **Recevoir les alertes sur cet ordinateur ?** avec
**Activer** (le navigateur demande l'autorisation, puis s'inscrit) et **Plus tard**. Le bandeau
n'apparaît que si le navigateur gère le Web Push et que l'autorisation n'a pas encore été
décidée ; une fois « Activer » ou « Plus tard » cliqué, il ne revient plus (drapeau
`notifications_proposees` du compte). La carte « Mon compte » reste pour changer plus tard.

### Pour les apps (Bearer `ekoa_…`)

- **Journal, pour Mac, Linux et Windows** : `GET /api/moi/notifications?apres=<id>` (interrogé
  chaque minute). Réponse `200` :
  `{ "notifications": [ { "id", "type", "site", "titre", "corps", "lien", "cree_le" } ], "dernier": <id> }`.
  `type` : `site_hors_ligne`, `site_en_ligne` ou `nouvelle_app` ; `site` : identifiant du site ou
  `null` ; `lien` : adresse absolue de la page à ouvrir ; `cree_le` : secondes Unix. Au plus 50
  notifications d'identifiant > `apres`, dans l'ordre croissant (si `dernier` dépasse le dernier
  `id` rendu, redemander avec `apres` = ce dernier `id`). `dernier` est le plus grand identifiant
  du compte (0 si le journal est vide), jamais réutilisé. **Sans `apres`**, la liste est vide et
  seul `dernier` est rendu : pas de rattrapage au premier lancement, l'app garde `dernier` puis
  interroge avec `apres=<dernier>`. `apres` non numérique : `400 {"code":"apres_invalide"}` ;
  jeton invalide : `401`. Le journal est gardé 30 jours (table `notifications`).
- **Push mobile (Expo)** : `POST /api/moi/push` avec
  `{ "jeton_expo": "ExponentPushToken[…]", "plateforme": "ios" | "android" }` → `204`
  (`400` avec `code` = `jeton_expo_invalide` ou `plateforme_invalide`, `401` sans jeton valide).
  Le jeton Expo est rattaché au jeton d'app : il disparaît avec lui (déconnexion de l'app,
  révocation dans Mon compte, compte supprimé), et un même appareil reconnecté avec un autre
  jeton d'app ne notifie qu'une fois. À rappeler à chaque lancement (le jeton Expo peut changer).
  `DELETE /api/moi/push` → `204` : cette app n'est plus notifiée. Envoi vers
  `https://exp.host/--/api/v2/push/send` :
  `{ "to", "title", "body", "sound": "default", "channelId": "alertes", "data": { "evenement": { "id", "le", "genre", "gravite": "alerte" | "info", "titre", "texte", "site", "lien" } } }`
  (`id` = identifiant du journal, `le` = secondes Unix, `genre` = `type` du journal ; gravité
  `alerte` pour hors ligne et nouvelle app, `info` pour le retour en ligne). Si Expo répond
  `DeviceNotRegistered`, le jeton est oublié. `EKO_CLOUD_EXPO_TOKEN` (facultatif, dans
  `/etc/default/eko-rip-cloud`) est le jeton d'accès Expo, envoyé en `Authorization: Bearer`.

### Web Push (navigateur)

Standard Web Push (RFC 8030) : messages chiffrés `aes128gcm` (RFC 8291), plate-forme identifiée
par VAPID (RFC 8292, JWT ES256, `sub` = `mailto:contact@e-ko.rip`, valable 12 h). Tout en Rust
(RustCrypto : `p256`, `hkdf`, `aes-gcm`, `sha2`), sans OpenSSL : la construction musl reste
possible. Les clés VAPID sont créées au premier démarrage et gardées dans `reglages` (la clé privée
n'est jamais affichée ni renvoyée).

- `GET /sw.js` : service worker à la racine (même origine), affiche titre, corps et icône
  `/logo-courriel.png`, ouvre `lien` au clic (même origine seulement).
- `GET /api/push/cle` : `{ "cle": <clé publique VAPID, base64url> }`.
- **Mon compte › Notifications sur cet appareil** : bouton **Activer les notifications du
  navigateur** et liste des navigateurs inscrits avec **Retirer** (10 au plus par compte).
  `POST /compte/push` (session du portail, `Origin` de la plate-forme et en-tête `X-CSRF-Token` ;
  corps JSON d'une `PushSubscription`) et `POST /compte/push/{id}/retirer` (formulaire à jeton
  anti-falsification). Seules les adresses d'abonnement HTTPS vers un nom d'hôte public sont
  acceptées. Réponse `404` ou `410` du service de notification : l'abonnement est supprimé.
- La CSP du portail autorise `connect-src 'self'` et `worker-src 'self'` pour le script
  d'inscription.

Tables de `portail.sqlite` : `notifications` (journal), `abonnements_web`, et deux colonnes de
`jetons_applis` (`jeton_expo`, `plateforme_expo`).

## Offres et abonnement

Prix hors taxes, utilisateurs illimités dans toutes les offres. L'abonnement annuel coûte
10 mois pour 12. Le détail est sur `/abonnement/offres` et dans les CGV (`/cgv`).

| Offre | Prix | Sites | Machines | Sauvegardes | Stockage |
|---|---|---|---|---|---|
| Gratuit | 0 € | 1 | 1 imprimante + 1 découpeur | dernière version | 1 Go |
| À la machine | 6 € / mois par machine au-delà des 2 gratuites | 1 | à la demande | 30 versions | 5 Go par machine |
| Atelier | 19 € / mois | 1 | 5 (+4 € / mois par machine en plus) | 90 jours | 25 Go |
| Pro | 49 € / mois | 3 | 15 (+4 € / mois par machine en plus) | 1 an, restauration vers un autre site | 100 Go |
| Entreprise | sur devis | | | | |

Les machines comptées sont celles que chaque serveur RIP déclare à la plate-forme (inventaire
envoyé au démarrage, puis régulièrement) : rien à saisir à la main. La page **Abonnement** les
liste site par site.

### Souscrire

1. **Abonnement › Voir les offres**, choisir l'offre et la périodicité (mensuelle ou annuelle).
2. Renseigner l'entreprise : le SIREN remplit la raison sociale et l'adresse ; contact et e-mail
   de facturation, téléphone mobile (pour le code du mandat SEPA), n° de bon de commande
   facultatif, code promotionnel ou code partenaire facultatif (vérifié aussitôt, ses conditions
   s'affichent). Cocher l'acceptation des CGV.
3. Payer, sans quitter le portail :
   - **Carte bancaire** : la carte est saisie dans le champ sécurisé du prestataire de paiement
     (la plate-forme ne voit jamais son numéro). Si la banque demande une authentification
     (3-D Secure), elle s'ouvre dans une fenêtre au-dessus de la page.
   - **Prélèvement SEPA** : saisir l'IBAN et le titulaire ; le mandat s'affiche dans la page,
     un code est envoyé par SMS, puis on signe en tapant son nom. Le mandat signé reste
     téléchargeable depuis la page Abonnement. Un mandat vaut pour l'entreprise renseignée : si
     elle change avant la signature, il faut en créer un nouveau.
4. La première souscription payante du compte, si c'est l'offre **Atelier**, ouvre droit à
   **30 jours d'essai** : aucune facture avant la fin de l'essai (la carte est seulement
   vérifiée, sans débit). L'essai n'est offert qu'une fois par compte, même après une
   résiliation. Un e-mail confirme l'abonnement, puis chaque facture émise.

Tous les e-mails (confirmation, factures, relances, fin d'essai) viennent de la plate-forme,
au contact de facturation ; les factures sont émises au début de chaque période.

### Changer d'offre, résilier, impayés

- **Changer d'offre ou de périodicité** : immédiat ; l'abonnement précédent s'arrête et une
  nouvelle période commence au prix de la nouvelle offre, avec le même moyen de paiement.
- **Changer de moyen de paiement** : depuis la page Abonnement (nouvelle carte ou nouveau mandat).
- **Résilier** : effet à la fin de la période en cours (non remboursée), puis retour à l'offre
  Gratuite.
- **Paiement refusé** : e-mail aussitôt, nouvelles tentatives, relances vers J+3, J+7 et J+14 ;
  sans régularisation, l'abonnement est résilié et le compte revient à l'offre Gratuite.

### Dépassement de l'offre

Quand le parc dépasse l'offre (site ou machines en trop), le propriétaire et les administrateurs
des sites sont prévenus (e-mail, notification, bandeau sur Mes sites et Abonnement). Ils ont
**14 jours** pour souscrire une offre adaptée ou retirer ce qui dépasse (30 jours après la mise
en service des offres pour les comptes existants). Passé ce délai, **l'accès à distance** des
sites concernés est suspendu (réponse `402 offre_depassee`, page explicative dans le
navigateur) ; le travail dans l'atelier n'est jamais interrompu. L'accès revient dès que le parc
rentre dans l'offre.

## Sauvegardes en ligne

Un serveur RIP rattaché peut envoyer à la plate-forme une sauvegarde de ses réglages (la nuit,
après un changement ou à la demande). Elles sont **chiffrées** et stockées chez un hébergeur en
France. Une sauvegarde identique à la précédente n'est pas stockée deux fois ; celle de la nuit
part quand même et confirme à la plate-forme que le serveur sauvegarde bien.

- **Mes sites › Sauvegardes** : liste des versions (date, motif, taille), place utilisée et
  quota de l'offre. Chaque version se télécharge, ou se **restaure** sur le serveur RIP (le
  serveur reçoit l'ordre par le tunnel, vérifie qu'il vient bien de la plate-forme et applique
  la sauvegarde). Avec l'offre **Pro**, une sauvegarde peut être restaurée vers un autre site
  du même propriétaire.
- Conservation selon l'offre : dernière version (Gratuit), 30 versions (À la machine), 90 jours
  (Atelier), 1 an (Pro). La version la plus récente est toujours gardée.
- Quota plein : la plate-forme prévient le propriétaire et refuse les nouvelles sauvegardes
  jusqu'à ce qu'il y ait de la place (offre supérieure ou versions supprimées par la rotation).
- Pas de sauvegarde de nuit reçue depuis plus de deux jours : e-mail au propriétaire et aux
  administrateurs du site. Le résumé hebdomadaire indique l'état des sauvegardes de chaque site.
- Retour à l'offre Gratuite : les versions existantes restent 90 jours, puis seule la dernière
  est gardée. Site retiré : ses sauvegardes sont supprimées 90 jours après la dernière.

Côté serveur RIP, l'échange se fait avec la clé du site (`Authorization: Bearer <site>.<clé>`) :

| Requête | Rôle |
|---|---|
| `POST /api/site/inventaire` | machines déclarées (`{version, imprimantes:[{id, nom, modele}], decoupeurs:[…]}`) ; réponse `{offre, sauvegardes, depassement}` |
| `POST /api/site/sauvegardes` | envoi d'une archive (200 Mo au plus), en-têtes `X-EKO-Motif` (`nuit`, `changement`, `demande`) et `X-EKO-Empreinte` (SHA-256) ; `201 {id}`, `409 identique`, `413 quota`, `402 offre` |
| `GET /api/site/sauvegardes` | versions disponibles pour ce site |
| `GET /api/site/sauvegardes/<id>` | contenu d'une version |

La restauration est un `POST /api/cloud/restaurer` (`{id}`) envoyé par la plate-forme dans le
tunnel, signé dans l'en-tête `X-EKO-Cloud-Service: t=<horodatage>,v1=<HMAC-SHA256>` (clé : empreinte
de la clé du site, message `<t>.<corps>`) ; le serveur refuse une signature fausse ou de plus de
5 minutes.

## Régler la facturation et le stockage (administration)

Tout se règle dans **Administration**, jamais dans un fichier : les clés et secrets saisis ne
sont jamais réaffichés (champ vide = valeur gardée, case « Effacer » pour la retirer). Chaque
carte a un bouton **Tester**.

- **Facturation** : adresse de l'API de l'outil de facturation et son jeton (droits tiers,
  abonnements, plans, codes partenaires, mandats SEPA et factures en lecture), secret de
  signature des webhooks, clé publique du prestataire de paiement (`pprod_…`, ou `ptest_…` en test ; la clé secrète `sprod_…` est refusée), et le code de plan
  de chaque offre et périodicité (À la machine, Atelier, Pro, machines supplémentaires ×
  mensuel / annuel). Une offre sans code de plan n'est pas proposée. Les plans se créent sans
  essai : la plate-forme demande elle-même les 30 jours de l'offre Atelier à la première
  souscription. Le webhook à déclarer dans
  l'outil de facturation est `https://platform.e-ko.rip/api/erp/webhooks` (événements abonnement,
  cycle de facturation, relance, mandat SEPA) ; les e-mails aux clients de l'outil de
  facturation restent désactivés, la plate-forme les envoie elle-même.
  **Enregistrer et configurer depuis l'ERP** fait le reste en un clic, avec un jeton qui a en
  plus le droit `webhooks.manage` (ERP 1.95.510 ou plus récent) : la plate-forme lit les plans,
  enregistre les codes `RIP-MACHINE-M`, `RIP-ATELIER-A`, `RIP-SUPP-M`… (`RIP-<OFFRE>-<M|A>`, SUPP
  pour les machines supplémentaires), crée le webhook avec tous ses événements et garde son
  secret sans l'afficher, puis supprime l'ancien webhook vers la même adresse. Après avoir
  régénéré le jeton dans l'ERP, il suffit de le coller et de relancer la configuration.
- **Stockage** : point d'accès S3, région, compartiment, clé d'accès et clé secrète, chiffrement
  côté serveur ; et un compartiment séparé pour la copie quotidienne de la base du portail.
- **Abonnements** : vue d'ensemble des comptes, de leur offre, de leurs machines et des
  dépassements en cours.

## Côté serveur RIP : inventaire, sauvegarde et restauration

L'offre se paie **par machine** (gratuit : 1 site, 1 imprimante et 1 découpeur). Pour que la
plate-forme les compte, le serveur RIP déclare ses machines (identifiant, nom, modèle ; les
imprimantes d'un côté, les traceurs et tables de découpe de l'autre) à l'ouverture du tunnel,
à chaque changement du catalogue et toutes les 6 heures. Une machine qui imprime (y compris
la Roland VersaCAMM, qui découpe elle-même) compte comme imprimante. La plate-forme répond
avec l'offre du site, et si elle inclut les sauvegardes ou si le site dépasse son forfait.

Les offres payantes incluent la **sauvegarde en ligne de la configuration** : la même archive
que Système › Sauvegarde (machines, réglages, profils ICC, règles, gabarits, tons directs,
tarifs, découpe…, ni les comptes ni les travaux), envoyée à la plate-forme qui la conserve. Le serveur la pousse :

- **chaque nuit**, à une minute tirée au hasard entre 1 h et 5 h (heure du serveur), seulement
  si la configuration a changé ;
- **2 minutes après un changement** de configuration ;
- **à la demande**, par un administrateur : `POST /api/cloud/sauvegarder`.

Sur une offre sans sauvegarde, le serveur n'insiste pas jusqu'au prochain inventaire qui l'autorise.
`GET /api/cloud` expose `sauvegarde` (`derniere` : date Unix ou `null`, `etat` : `ok`, `offre`,
`quota` ou `erreur`, `message`) et `abonnement` (`offre`, `sauvegardes`, `depassement`).

**Restaurer** : depuis le portail, la plate-forme envoie par le tunnel `POST /api/cloud/restaurer`
`{"id": "…"}`, signé (`X-EKO-Cloud-Service: t=<unix>,v1=<hex>`, HMAC-SHA256 avec le SHA-256 de
la clé du site, message `<t>.<corps>`, tolérance 300 s, une signature ne sert qu'une fois). Le
serveur télécharge l'archive et l'applique comme `POST /api/sauvegarde`. Sans signature valide :
401 `signature_cloud_invalide`, session ou non.

## Ce qui ne passe jamais par le cloud

Une requête arrivée par le tunnel est marquée par le serveur lui-même (une requête du réseau
local ne peut pas porter cette marque). Le serveur refuse alors, avec le code `ferme_par_cloud` :

- les routes du pont ERP et le renvoi entre RIP (`/api/renvoi/…`) ;
- les réglages du pont ERP (`/api/pont…`) et les changements du cloud lui-même (appairage,
  détachement : à faire depuis l'atelier), sauf l'ordre de restauration signé de la plate-forme.

Sans compte déclaré, toute requête venue par le cloud est refusée (`cloud_sans_compte`).

## Sécurité côté plate-forme

- La clé d'un site (32 octets aléatoires) n'est gardée que sous forme d'empreinte SHA-256.
  Site inconnu ou mauvaise clé : même réponse (`cle_invalide`).
- Vers un atelier ne part que la session du serveur RIP (cookie `eko_rip_session`) : ni
  l'en-tête `Authorization`, ni les cookies du portail, ni les en-têtes `X-EKO-Cloud-*`
  (réservés à la plate-forme).
- Chaque site a sa propre origine (`<site>.platform.e-ko.rip`) : le navigateur sépare ses
  cookies et ses pages de ceux des autres sites et du portail. Le `Domain` des cookies posés par
  un serveur RIP est retiré ; les cookies du portail portent le préfixe `__Host-`, qu'un
  sous-domaine ne peut ni poser ni écraser. Le portail n'existe pas sur l'adresse d'un site.
- Sous l'ancienne forme `/s/<site>/`, le cookie d'un site est limité à ce chemin et toute
  réponse porte `Content-Security-Policy: sandbox` (pas de script, origine opaque).
- Requête de plus de 2 Go (`EKO_CLOUD_MAX_MO`) : refusée (`envoi_trop_gros`). Site hors ligne
  ou inconnu : même réponse (`503 site_hors_ligne`).

## Installer la plate-forme (VPS)

Paquet `eko-rip-cloud` (`cargo deb -p eko-rip-cloud`), service systemd `eko-rip-cloud`, compte
système dédié, données dans `/var/lib/eko-rip-cloud` (`sites.json`, `validations/`). Il écoute
sur `127.0.0.1:8650` ; Caddy fait le HTTPS :

```caddy
{
    # Certificat de chaque site à sa première visite, seulement s'il existe.
    on_demand_tls {
        ask http://127.0.0.1:8650/api/certificat
    }
}

platform.e-ko.rip, *.platform.e-ko.rip {
    tls {
        on_demand
    }
    # Le tunnel (WebSocket) passe tel quel ; les flux ne sont pas mis en tampon.
    reverse_proxy 127.0.0.1:8650 {
        flush_interval -1
    }
}
```

DNS : `platform.e-ko.rip` **et** `*.platform.e-ko.rip` pointent vers l'hébergement de la plate-forme.
`GET /api/certificat?domain=<nom>` répond 200 pour la plate-forme et les sites existants, 404
sinon, ce qui empêche d'obtenir des certificats pour des noms inventés. Let's Encrypt limite à
50 nouveaux certificats par semaine pour `e-ko.rip` : au-delà, passer à un certificat joker
(`*.platform.e-ko.rip`, défi DNS chez OVH).

Commandes : `eko-rip-cloud admin <e-mail>` (super-administrateur), `eko-rip-cloud sites` (liste), `eko-rip-cloud valider <code> [propriétaire]`,
`eko-rip-cloud supprimer <site>` (son tunnel est coupé dans la minute). Réglages dans
`/etc/default/eko-rip-cloud` : `EKO_CLOUD_ADDR`, `EKO_CLOUD_URL`, `EKO_CLOUD_MAX_MO` (et `EKO_CLOUD_SMTP_*`, voir « E-mails de la plate-forme »).

## Installer la plate-forme (hébergement mutualisé o2switch)

o2switch (cPanel, LiteSpeed) ne lance que des applications Node.js. Le lanceur
`cloud/o2switch/app.js` (sans dépendance npm) démarre le relais sur un port local, relance
le relais s'il s'arrête et lui passe les requêtes comme le WebSocket du tunnel.

1. **Binaire.** Le serveur mutualisé a une glibc ancienne : construire en statique (musl).
   ```bash
   rustup target add x86_64-unknown-linux-musl   # une fois (+ apt install musl-tools)
   cargo build --release --locked --target x86_64-unknown-linux-musl -p eko-rip-cloud --bin eko-rip-cloud
   ```
2. **Envoi.** Créer `~/eko-rip-cloud/` sur le compte et y copier `app.js` et le binaire
   (`target/x86_64-unknown-linux-musl/release/eko-rip-cloud`, droits 755). Le SSH n'est
   ouvert qu'aux adresses déclarées dans cPanel (« Autorisation SSH »).
3. **Domaines.** cPanel → Domaines : ajouter `platform.e-ko.rip` avec pour racine
   `~/eko-rip-cloud/public`. DNS chez OVH : `platform` en A vers l'adresse du serveur
   o2switch. Puis cPanel → « Lets Encrypt SSL » pour `platform.e-ko.rip`.
4. **Application.** cPanel → « Setup Node.js App » → Create : Node 22, mode Production,
   racine `eko-rip-cloud`, URL `platform.e-ko.rip`, fichier de démarrage `app.js`. Variables :
   `EKO_CLOUD_URL=https://platform.e-ko.rip`, `EKO_CLOUD_DATA=/home/<compte>/eko-cloud-data`
   (hors de la racine web), et si besoin `EKO_CLOUD_JETON_STATS`. Start. Le relais écoute sur
   un socket local (`~/eko-rip-cloud/relais.sock`, `EKO_CLOUD_ADDR=unix:<chemin>`) et non sur
   un port : o2switch arrête en quelques secondes tout programme du compte qui ouvre un port.
5. **Premier compte.** En SSH, dans `~/eko-rip-cloud` :
   `EKO_CLOUD_DATA=~/eko-cloud-data ./eko-rip-cloud admin <e-mail>`.
6. **Vérifier.** `curl https://platform.e-ko.rip/sante` répond `ok`, puis rattacher un serveur
   RIP (le tunnel doit rester « connecté » plusieurs minutes).

Une adresse par site (`<site>.platform.e-ko.rip`) : ajouter aussi le sous-domaine joker
`*.platform.e-ko.rip` (même racine, rattaché à la même application), DNS `*.platform` en A vers
le serveur o2switch, et un certificat qui couvre le joker. Sans lui, les clients joignent les
sites à `https://platform.e-ko.rip/s/<site>/` (l'interface web dans un navigateur, elle,
demande l'adresse propre au site).

Le frontal d'o2switch remplace toute réponse 502 de l'application par une page « 421
Misdirected Request » : le relais et le lanceur répondent donc 503 quand un site ou le relais
est indisponible. Le journal du relais est `~/eko-rip-cloud/relais.log`.

Mise à jour : remplacer le binaire, puis « Restart » dans « Setup Node.js App ».

## Protocole du tunnel

Le serveur RIP envoie `GET /tunnel` (HTTP/1.1) avec `Authorization: Bearer <site>.<clé>`,
`Upgrade: websocket` et `Sec-WebSocket-Protocol: eko-tunnel/1` (WebSocket ordinaire, RFC
6455). La plate-forme répond `101 Switching Protocols` ; dans les messages binaires du
WebSocket, elle parle HTTP/2 **en client** et le serveur RIP sert sa propre API. Plusieurs
requêtes à la fois, fichiers en flux, ping toutes les 25 s. Le WebSocket ne sert qu'à passer
les frontaux qui refusent tout autre `Upgrade` (LiteSpeed, Passenger). La plate-forme accepte
encore l'ancienne forme, `Upgrade: eko-tunnel/1` seul, des serveurs RIP 0.30.

**Sens montant en requêtes POST (`eko-tunnel/2`).** Certains frontaux d'hébergement mutualisé
(celui d'o2switch) laissent passer le WebSocket dans un seul sens : ce que le serveur RIP
envoie après la poignée de main reste dans leur tampon. Le serveur RIP propose donc
`Sec-WebSocket-Protocol: eko-tunnel/2, eko-tunnel/1`. Quand la plate-forme est réglée pour
(`EKO_CLOUD_MONTANT_POST=1`, posé par défaut par le lanceur o2switch), elle choisit
`eko-tunnel/2` :

- son premier message dans le WebSocket est un texte `session <id>` (32 caractères
  hexadécimaux, aléatoires) ; le WebSocket ne porte ensuite que le sens plate-forme → RIP ;
- le serveur RIP envoie le sens montant par `POST /tunnel/<id>/montant`, en-tête
  `X-EKO-Seq: 0, 1, 2…`, corps de 1 Mio au plus, sur une connexion HTTP/1.1 gardée ouverte.
  Réponse `204` ; un numéro déjà reçu est acquitté sans effet (renvoi après une coupure) ;
  un numéro en avance ou une session inconnue rend `409 session_perdue` et le serveur RIP
  rouvre le tunnel.

Sinon la plate-forme répond `eko-tunnel/1` et le WebSocket porte les deux sens. Les serveurs
RIP 0.31 et plus anciens ne proposent que `eko-tunnel/1`.

Appairage : `POST /api/appairage` `{ "nom": "…" }` → `201 { demande, code, lien, expire_dans }` ;
`GET /api/appairage/<demande>` → `{ "etat": "en_attente" }`, puis une seule fois
`{ "etat": "valide", "site", "cle", "adresse" }`, puis `404 appairage_expire`.
