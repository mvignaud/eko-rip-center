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
- **Mes sites** : état en ligne / hors ligne, adresse à distance, rôle ; rattacher (10 sites par
  compte), retirer (le propriétaire ; tunnel coupé aussitôt).
- **Mon compte** : connexions liées, suppression du compte (et des sites possédés).
- **Administration** (super-administrateur, donné par `eko-rip-cloud admin <e-mail>` sur le VPS) :
  clés Google et Facebook (les secrets ne sont jamais réaffichés), avec les URL à déclarer chez
  eux (redirection, confidentialité, CGU, suppression des données) ; liste des sites (retirer) et
  des comptes (suspendre : ses sessions sont fermées).
- **Pages légales** : `/mentions-legales`, `/cgu`, `/confidentialite` (éditeur SAS GOLDEN PRINT ;
  RCS Auch 939 018 024, contact contact@eko.rip, pas de téléphone).
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
- Chaque site a sa propre origine (`<site>.platform.e-ko.rip`) : le navigateur sépare ses
  cookies et ses pages de ceux des autres sites et du portail. Le `Domain` des cookies posés par
  un serveur RIP est retiré ; les cookies du portail portent le préfixe `__Host-`, qu'un
  sous-domaine ne peut ni poser ni écraser. Le portail n'existe pas sur l'adresse d'un site.
- Sous l'ancienne forme `/s/<site>/`, le cookie d'un site est limité à ce chemin et toute
  réponse porte `Content-Security-Policy: sandbox` (pas de script, origine opaque).
- Requête de plus de 2 Go (`EKO_CLOUD_MAX_MO`) : refusée (`envoi_trop_gros`). Site hors ligne
  ou inconnu : même réponse (`site_hors_ligne`).

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

DNS : `platform.e-ko.rip` **et** `*.platform.e-ko.rip` pointent vers le VPS (A 54.36.102.38).
`GET /api/certificat?domain=<nom>` répond 200 pour la plate-forme et les sites existants, 404
sinon, ce qui empêche d'obtenir des certificats pour des noms inventés. Let's Encrypt limite à
50 nouveaux certificats par semaine pour `e-ko.rip` : au-delà, passer à un certificat joker
(`*.platform.e-ko.rip`, défi DNS chez OVH).

Commandes : `eko-rip-cloud admin <e-mail>` (super-administrateur), `eko-rip-cloud sites` (liste), `eko-rip-cloud valider <code> [propriétaire]`,
`eko-rip-cloud supprimer <site>` (son tunnel est coupé dans la minute). Réglages dans
`/etc/default/eko-rip-cloud` : `EKO_CLOUD_ADDR`, `EKO_CLOUD_URL`, `EKO_CLOUD_MAX_MO`.

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
   (hors de la racine web), et si besoin `EKO_CLOUD_PORT_INTERNE` (8650 par défaut, à changer
   s'il est pris sur le serveur), `EKO_CLOUD_JETON_STATS`. Start.
5. **Premier compte.** En SSH, dans `~/eko-rip-cloud` :
   `EKO_CLOUD_DATA=~/eko-cloud-data ./eko-rip-cloud admin <e-mail>`.
6. **Vérifier.** `curl https://platform.e-ko.rip/sante` répond `ok`, puis rattacher un serveur
   RIP (le tunnel doit rester « connecté » plusieurs minutes).

Une adresse par site (`<site>.platform.e-ko.rip`) demande un sous-domaine joker et son
certificat joker, que Let's Encrypt ne délivre qu'avec un défi DNS. Tant que ce certificat
n'est pas en place, les clients joignent les sites à `https://platform.e-ko.rip/s/<site>/`
(l'interface web dans un navigateur, elle, demande l'adresse propre au site).

Mise à jour : remplacer le binaire, puis « Restart » dans « Setup Node.js App ».

## Protocole du tunnel

Le serveur RIP envoie `GET /tunnel` (HTTP/1.1) avec `Authorization: Bearer <site>.<clé>`,
`Upgrade: websocket` et `Sec-WebSocket-Protocol: eko-tunnel/1` (WebSocket ordinaire, RFC
6455). La plate-forme répond `101 Switching Protocols` ; dans les messages binaires du
WebSocket, elle parle HTTP/2 **en client** et le serveur RIP sert sa propre API. Plusieurs
requêtes à la fois, fichiers en flux, ping toutes les 25 s. Le WebSocket ne sert qu'à passer
les frontaux qui refusent tout autre `Upgrade` (LiteSpeed, Passenger). La plate-forme accepte
encore l'ancienne forme, `Upgrade: eko-tunnel/1` seul, des serveurs RIP 0.30.

Appairage : `POST /api/appairage` `{ "nom": "…" }` → `201 { demande, code, lien, expire_dans }` ;
`GET /api/appairage/<demande>` → `{ "etat": "en_attente" }`, puis une seule fois
`{ "etat": "valide", "site", "cle", "adresse" }`, puis `404 appairage_expire`.
