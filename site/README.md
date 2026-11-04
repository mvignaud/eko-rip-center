# Site vitrine e-ko.rip

Site statique (HTML, CSS, JavaScript sans dépendance ni étape de construction), servi par
Apache sur l'hébergement o2switch. Aucune ressource externe ni cookie, sauf le chat en direct
(`assets/chat.js`, support Suite E-KO sur help.suite-eko.com, autorisé dans la CSP de `.htaccess`).

| Page | Fichier |
|---|---|
| Accueil : maquettes Mac / Windows / Linux, fonctionnalités, fonctionnement, open source, installation | `index.html` |
| Pré-requis techniques (serveur, postes, réseau) | `prerequis/index.html` |
| Compatibilité machines et demande de pilote | `compatibilite/index.html`, `compatibilite/demande.php` |
| Journal des versions | `changelog/index.html` |
| Application mobile, confidentialité | `app/index.html`, `app/confidentialite/index.html` |
| Cloud E-KO RIP : présentation, tarifs des offres, liens vers platform.e-ko.rip | `cloud/index.html` |
| Mentions légales | `mentions-legales/index.html` |
| Documentation (FR, EN) : générée, ne pas modifier à la main | `documentation/`, `assets/documentation.css`, `assets/documentation.js` |
| Page 404 | `404.html` |
| Télécharger : serveur, clients, empreintes des fichiers | `telecharger/index.html`, `telecharger/telecharger.php` |
| Contact | `contact/index.html`, `contact/envoyer.php` |
| Blog (généré depuis `blog/fr/*.md`), flux RSS | `blog/` |
| Feuille de route (générée depuis `pages/feuille-de-route.md`) | `feuille-de-route/` |
| Plan du site, résumé pour les IA (générés) | `sitemap.xml`, `llms.txt` |

Styles dans `assets/style.css` (couleurs CMJN de l'icône, mode sombre automatique),
comportements dans `assets/site.js`, journal dans `assets/journal.js`.

## Menu, pied de page, blog, feuille de route

Le menu (4 rubriques, Contact, bouton Télécharger) et le pied de page sont définis une seule fois
dans `outils/site_commun.py`. `python3 outils/generer-site.py` les pose dans chaque page écrite à
la main, entre `<!-- entete -->`/`<!-- /entete -->` et `<!-- pied -->`/`<!-- /pied -->` (une
nouvelle page reprend ces repères vides), et la documentation les reprend aussi. Le même outil
génère le blog (`blog/README.md`), les trois derniers articles de l'accueil
(`<!-- blog-recents -->`), la feuille de route, `sitemap.xml` et `llms.txt`, et refuse un lien
interne cassé. `--verifier` (CI, `verifier.sh`, `publier-site.sh`) échoue si `site/` est en retard.

## Documentation

`documentation/` est générée depuis `docs/guide/` (et `docs/fr|en/`) par
`python3 outils/generer-documentation.py` : voir `docs/guide/README.md`. `publier-site.sh` refuse de
publier si elle est en retard sur sa source.

## Journal des versions

La source reste dans l'ERP : `resources/changelog-rip/X.Y.Z.yaml` d'eko-source, écrit par
`/changelog-rip` à chaque release. Le site le lit en direct sur
`https://app.e-ko.fr/changelog-rip.json?lang=fr|en` (CORS ouvert à e-ko.rip seulement) et
retombe sur la copie `changelog.<langue>.json` déposée par `outils/publier-site.sh`.
L'abonnement du site appelle `POST https://app.e-ko.fr/changelog-rip/subscribe` : c'est
toujours l'ERP qui confirme l'adresse et envoie l'e-mail à son déploiement. Une nouvelle
version apparaît donc sur le site sans le republier.

## Formulaires (demande de pilote, contact)

Deux scripts, `compatibilite/demande.php` et `contact/envoyer.php`, sur le code commun de
`inc/formulaire.php` (dossier jamais servi, `inc/.htaccess`), en PHP d'o2switch. Chacun reçoit
son formulaire (`/compatibilite/`, `/contact/`), l'envoie par `mail()` à `contact@e-ko.rip`
(Reply-To = le visiteur) et ne garde rien, sauf un compteur par IP hachée (5 envois par heure, dans le
dossier temporaire de PHP). Pot de miel `website`, origine `https://e-ko.rip` seule, champs
coupés et nettoyés des retours à la ligne. Avec JavaScript, la page reçoit la réponse en
JSON ; sans, le script renvoie sur `#demande-envoyee` / `#demande-erreur` (`#message-envoye` /
`#message-erreur` pour le contact), message affiché par `:target`, sans JavaScript.

Les chiffres des pré-requis et la liste des machines suivent `docs/fr/installation-serveur.md`
et `docs/fr/machines.md` : les mettre à jour ensemble.

## Essayer en local

```bash
cd site && python3 -m http.server 8765   # puis http://localhost:8765
```

## Publier

```bash
EKO_SITE_HOTE=<compte>@<hôte> outils/publier-site.sh
```

Le CSS et le JS sont mis en cache une heure (`.htaccess`), les pages jamais. Le script
suffixe donc à la publication les liens vers chaque `.css` et `.js` de `assets/` d'une
empreinte de leur contenu (`?v=…`) : rien à changer à la main.

Hôte (`EKO_SITE_HOTE`, obligatoire : compte SSH o2switch ou alias de `~/.ssh/config`) et dossier racine du
domaine dans cPanel (`EKO_SITE_DOSSIER`, défaut `public_html`, racine du domaine e-ko.rip chez o2switch) se changent par variables.
L'accès SSH d'o2switch n'est ouvert qu'aux adresses IP déclarées dans cPanel
(« Autorisation SSH »).

Captures : `assets/img/*.webp`, tirées des captures réelles du logiciel. Pour en ajouter
une, la convertir en WebP (qualité 85) et lui donner `width`, `height` et un `alt`.

## Téléchargement direct des clients

`/telecharger/mac` (règle dans `.htaccess`, script `telecharger/telecharger.php`) renvoie vers le
`.dmg` de la dernière release GitHub qui en a un. `/telecharger/windows` renvoie vers la PAGE de
la release, jamais vers le `.exe` tant qu'il n'est pas signé (Google classe un exécutable non
signé en un clic dans les « pages trompeuses »). `/telecharger/infos.json` donne version, taille
et empreinte SHA-256 des fichiers, affichées sur `/telecharger/`. La liste des releases est gardée
10 minutes en cache dans `~/eko-rip-cache` (hors du dossier public, pas dans le `/tmp` commun) ; sans réponse de GitHub, on renvoie vers la page des versions. Rien à
changer à chaque version : il suffit de joindre le fichier à la release.
