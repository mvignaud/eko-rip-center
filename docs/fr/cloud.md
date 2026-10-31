# Cloud E-KO RIP (accès à distance)

La plate-forme **E-KO RIP Cloud** (`https://platform.e-ko.rip`) donne accès à un serveur RIP
d'atelier depuis l'extérieur, **sans VPN et sans ouvrir de port** : c'est le serveur RIP qui
ouvre lui-même une connexion sortante (HTTPS, port 443) vers la plate-forme, et la plate-forme
fait passer par elle les requêtes des postes et de l'app mobile.

Service gratuit, ouvert à tout serveur E-KO RIP Center. État : **lots 1 et 2** (tunnel,
appairage, portail). À distance, on se connecte encore avec **les comptes du serveur RIP**,
comme à l'atelier ; l'identité de la plate-forme passera jusqu'au serveur au lot 3.

```
Poste ou téléphone hors atelier
   │ HTTPS  platform.e-ko.rip/s/<site>/api/…
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
   `https://platform.e-ko.rip/s/<site>`.

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
- **Mes sites** : état en ligne / hors ligne, adresse à distance, rôle ; rattacher (10 sites par
  compte), retirer (le propriétaire ; tunnel coupé aussitôt).
- **Mon compte** : connexions liées, suppression du compte (et des sites possédés).
- **Administration** (super-administrateur, donné par `eko-rip-cloud admin <e-mail>` sur le VPS) :
  clés Google et Facebook (les secrets ne sont jamais réaffichés), avec les URL à déclarer chez
  eux (redirection, confidentialité, CGU, suppression des données) ; liste des sites (retirer) et
  des comptes (suspendre : ses sessions sont fermées).
- **Pages légales** : `/mentions-legales`, `/cgu`, `/confidentialite` (éditeur SAS GOLDEN PRINT ;
  RCS Auch 939 018 024 ; restent à remplir les e-mails de contact et du référent données et le téléphone,
  marqués « À COMPLÉTER » dans `cloud/static/legal/`).
- Protections : formulaires acceptés seulement depuis la plate-forme (`Origin`) avec un jeton par
  session ; 5 échecs de mot de passe bloquent le compte 5 minutes ; limites par adresse
  (inscriptions, connexions) ; cookie de session `HttpOnly`, `SameSite=Lax`, jamais transmis
  aux ateliers ; pages servies avec une politique de sécurité stricte (CSP, pas d'iframe).
- Données : `/var/lib/eko-rip-cloud/portail.sqlite` (0600) à côté de `sites.json`.

## Utiliser l'accès à distance

Dans un client (Mac, Linux, Windows, mobile), l'adresse du serveur devient
`https://platform.e-ko.rip/s/<site>` ; la connexion se fait avec un compte du serveur RIP. Les
clients se brancheront directement sur la plate-forme au lot 4 (connexion au compte du cloud,
bascule automatique entre l'adresse locale et l'accès à distance).

## Ce qui ne passe jamais par le cloud

Une requête arrivée par le tunnel est marquée par le serveur lui-même (une requête du réseau
local ne peut pas porter cette marque). Le serveur refuse alors, avec le code `ferme_par_cloud` :

- les routes du pont ERP et le renvoi entre RIP (`/api/renvoi/…`) ;
- les réglages du pont ERP (`/api/pont…`) et les changements du cloud lui-même (appairage,
  détachement : à faire depuis l'atelier).

Sans compte déclaré, toute requête venue par le cloud est refusée (`cloud_sans_compte`).

## Sécurité côté plate-forme

- La clé d'un site (32 octets aléatoires) n'est gardée que sous forme d'empreinte SHA-256.
  Site inconnu ou mauvaise clé : même réponse (`cle_invalide`).
- Vers un atelier ne part que la session du serveur RIP (cookie `eko_rip_session`) : ni
  l'en-tête `Authorization`, ni les cookies du portail, ni les en-têtes `X-EKO-Cloud-*`
  (réservés à la plate-forme).
- Le cookie de session d'un site est limité à son chemin `/s/<site>/` (`Secure` en HTTPS) :
  un site ne reçoit jamais la session d'un autre.
- Toute réponse d'un atelier porte `Content-Security-Policy: sandbox` : une page servie par un
  serveur RIP n'a pas l'origine de la plate-forme et ne peut rien y lire.
- Requête de plus de 2 Go (`EKO_CLOUD_MAX_MO`) : refusée (`envoi_trop_gros`). Site hors ligne
  ou inconnu : même réponse (`site_hors_ligne`).

## Installer la plate-forme (VPS)

Paquet `eko-rip-cloud` (`cargo deb -p eko-rip-cloud`), service systemd `eko-rip-cloud`, compte
système dédié, données dans `/var/lib/eko-rip-cloud` (`sites.json`, `validations/`). Il écoute
sur `127.0.0.1:8650` ; Caddy fait le HTTPS :

```caddy
platform.e-ko.rip {
    # Le tunnel (Upgrade: eko-tunnel/1) passe tel quel ; les flux ne sont pas mis en tampon.
    reverse_proxy 127.0.0.1:8650 {
        flush_interval -1
    }
}
```

Commandes : `eko-rip-cloud admin <e-mail>` (super-administrateur), `eko-rip-cloud sites` (liste), `eko-rip-cloud valider <code> [propriétaire]`,
`eko-rip-cloud supprimer <site>` (son tunnel est coupé dans la minute). Réglages dans
`/etc/default/eko-rip-cloud` : `EKO_CLOUD_ADDR`, `EKO_CLOUD_URL`, `EKO_CLOUD_MAX_MO`.

## Protocole du tunnel

Le serveur RIP envoie `GET /tunnel` (HTTP/1.1) avec `Authorization: Bearer <site>.<clé>` et
`Upgrade: eko-tunnel/1`. La plate-forme répond `101 Switching Protocols` ; dans la connexion
ainsi obtenue, elle parle HTTP/2 **en client** et le serveur RIP sert sa propre API. Plusieurs
requêtes à la fois, fichiers en flux, ping toutes les 25 s.

Appairage : `POST /api/appairage` `{ "nom": "…" }` → `201 { demande, code, lien, expire_dans }` ;
`GET /api/appairage/<demande>` → `{ "etat": "en_attente" }`, puis une seule fois
`{ "etat": "valide", "site", "cle", "adresse" }`, puis `404 appairage_expire`.
