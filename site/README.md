# Site vitrine e-ko.rip

Site statique (HTML, CSS, JavaScript sans dépendance ni étape de construction), servi par
Apache sur l'hébergement o2switch. Aucune ressource externe, aucun cookie.

| Page | Fichier |
|---|---|
| Accueil : maquettes Mac / Windows / Linux, fonctionnalités, fonctionnement, open source, installation | `index.html` |
| Pré-requis techniques (serveur, postes, réseau) | `prerequis/index.html` |
| Compatibilité machines et demande de pilote | `compatibilite/index.html`, `compatibilite/demande.php` |
| Journal des versions | `changelog/index.html` |
| Application mobile, confidentialité | `app/index.html`, `app/confidentialite/index.html` |
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

## Demande de pilote

Seul script du site : `compatibilite/demande.php` (PHP d'o2switch). Il reçoit le formulaire
de `/compatibilite/`, envoie la demande par `mail()` à `contact@e-ko.rip` (Reply-To = le
demandeur) et ne garde rien, sauf un compteur par IP hachée (5 envois par heure, dans le
dossier temporaire de PHP). Pot de miel `website`, origine `https://e-ko.rip` seule, champs
coupés et nettoyés des retours à la ligne. Avec JavaScript, la page reçoit la réponse en
JSON ; sans, le script renvoie sur `/compatibilite/#demande-envoyee` ou `#demande-erreur` (message affiché par `:target`, sans JavaScript).

Les chiffres des pré-requis et la liste des machines suivent `docs/fr/installation-serveur.md`
et `docs/fr/machines.md` : les mettre à jour ensemble.

## Essayer en local

```bash
cd site && python3 -m http.server 8765   # puis http://localhost:8765
```

## Publier

```bash
outils/publier-site.sh
```

Le CSS et le JS sont mis en cache une heure (`.htaccess`), les pages jamais. Le script
suffixe donc à la publication les liens vers `style.css`, `site.js` et `journal.js` d'une
empreinte de leur contenu (`?v=…`) : rien à changer à la main, mais une nouvelle ressource
CSS ou JS doit être ajoutée à sa boucle.

Hôte (`EKO_SITE_HOTE`, défaut `sc3nihe1770@fleur.o2switch.net`) et dossier racine du
domaine dans cPanel (`EKO_SITE_DOSSIER`, défaut `public_html`, racine du domaine e-ko.rip chez o2switch) se changent par variables.
L'accès SSH d'o2switch n'est ouvert qu'aux adresses IP déclarées dans cPanel
(« Autorisation SSH »).

Captures : `assets/img/*.webp`, tirées des captures réelles du logiciel. Pour en ajouter
une, la convertir en WebP (qualité 85) et lui donner `width`, `height` et un `alt`.
