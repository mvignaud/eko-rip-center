# Site vitrine e-ko.rip

Site statique (HTML, CSS, JavaScript sans dépendance ni étape de construction), servi par
Apache sur l'hébergement o2switch. Aucune ressource externe, aucun cookie.

| Page | Fichier |
|---|---|
| Accueil : maquettes Mac / Windows / Linux, fonctionnalités, fonctionnement, open source, installation | `index.html` |
| Journal des versions | `changelog/index.html` |
| Mentions légales | `mentions-legales/index.html` |
| Page 404 | `404.html` |

Styles dans `assets/style.css` (couleurs CMJN de l'icône, mode sombre automatique),
comportements dans `assets/site.js`, journal dans `assets/journal.js`.

## Journal des versions

La source reste dans l'ERP : `resources/changelog-rip/X.Y.Z.yaml` d'eko-source, écrit par
`/changelog-rip` à chaque release. Le site le lit en direct sur
`https://app.e-ko.fr/changelog-rip.json?lang=fr|en` (CORS ouvert à e-ko.rip seulement) et
retombe sur la copie `changelog.<langue>.json` déposée par `outils/publier-site.sh`.
L'abonnement du site appelle `POST https://app.e-ko.fr/changelog-rip/subscribe` : c'est
toujours l'ERP qui confirme l'adresse et envoie l'e-mail à son déploiement. Une nouvelle
version apparaît donc sur le site sans le republier.

## Essayer en local

```bash
cd site && python3 -m http.server 8765   # puis http://localhost:8765
```

## Publier

```bash
outils/publier-site.sh
```

Hôte (`EKO_SITE_HOTE`, défaut `sc3nihe1770@fleur.o2switch.net`) et dossier racine du
domaine dans cPanel (`EKO_SITE_DOSSIER`, défaut `e-ko.rip`) se changent par variables.
L'accès SSH d'o2switch n'est ouvert qu'aux adresses IP déclarées dans cPanel
(« Autorisation SSH »).

Captures : `assets/img/*.webp`, tirées des captures réelles du logiciel. Pour en ajouter
une, la convertir en WebP (qualité 85) et lui donner `width`, `height` et un `alt`.
