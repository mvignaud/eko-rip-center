# E-KO RIP Center — consignes pour Claude

## Release : publier sur le dépôt APT (obligatoire)

Chaque release se termine par la publication des paquets Linux sur le dépôt APT
`https://apt.e-ko.fr/rip/`. C'est par lui que les postes Linux et les serveurs RIP se mettent
à jour (`sudo apt update && sudo apt upgrade`). Une release qui n'est pas sur le dépôt n'arrive
chez personne.

Depuis le 2026-11-03, tout part du **serveur CI dédié** (OVH, 15.235.86.71, runner
`eko-ci-rip`), et seulement quand Mathieu l'écrit dans le chat (« Déploie le RIP », « Déploie
le site ») : Claude lance le workflow par l'API GitHub
(`POST /repos/mvignaud/eko-rip-center/actions/workflows/<fichier>/dispatches {"ref":"main"}`).

- **`release.yml`** (« Déploie le RIP ») : contrôle (version identique dans `Cargo.toml` et
  `clients/linux/Cargo.toml`, section du `CHANGELOG.md`, tag libre, `outils/verifier.sh`),
  construit `eko-rip-serveur`, `eko-rip-cloud` (Ubuntu 22.04) et le client Linux natif
  `e-ko-rip-center` (GTK 4, Ubuntu 26.04), crée le tag `vX.Y.Z` et la release GitHub, puis
  publie les trois `.deb` sur le dépôt APT avec `outils/publier-depot.sh` et vérifie
  `Packages`. Restent hors CI : client Mac signé et notarisé (Mac), client Windows, journal
  `/changelog-rip` de l'ERP (PR eko-source) ; les joindre ensuite (`gh release upload`).
- **`deploy-cloud.yml`** : plate-forme platform.e-ko.rip sur o2switch (binaire musl,
  sauvegarde, bascule, `/sante`, retour arrière sinon).
- **`deploy-site.yml`** : site e-ko.rip et documentation (`outils/publier-site.sh`).
- Clés en secrets de l'environnement GitHub `production` : `DEPOT_GPG_KEY` (clé privée
  `depot@e-ko.fr` exportée du serveur de test ; **ne jamais en régénérer une**, les postes
  refuseraient les mises à jour), `DEPOT_SSH_KEY` et `DEPOT_KNOWN_HOSTS` (compte `depot`
  d'apt.e-ko.fr, limité à `/srv/depot`), `O2SWITCH_SSH_KEY` et `O2SWITCH_KNOWN_HOSTS`. L'IP
  du serveur CI doit être autorisée dans cPanel d'o2switch (« Autorisation SSH »).
- Dépôt public : les workflows ne tournent jamais pour une PR venue d'un fork.
- Repli manuel (serveur de test `mathieu@eko-rip`, 192.168.10.109) :
  `~/eko-depot/outils/publier-depot.sh eko-rip-serveur_<version>-1_amd64.deb e-ko-rip-center_<version>-1_amd64.deb`
  (`~/eko-depot/outils/` est une copie de `outils/` : la recopier quand on modifie les scripts).
- Vérification : `curl -s https://apt.e-ko.fr/rip/Packages | grep -A1 '^Package:'` doit lister
  les nouvelles versions.
- Le VPS est décrit dans eko-infra (`ansible/depot.yml`).

## Documentation publique : à jour à chaque changement (obligatoire)

La documentation utilisateur est publiée sur `https://e-ko.rip/documentation/` (FR) et
`/documentation/en/` (EN). Source : `docs/guide/fr|en/` + références `docs/fr|en/`
(utilisation, machines, glossaire) ; pages générées dans `site/documentation/`.

- Toute PR qui change ce que voit ou fait un utilisateur (serveur, interface, clients Mac,
  Linux, Windows, app mobile, Cloud, installation) met à jour la doc **dans la même PR**, en
  français et en anglais, refait les captures touchées (`docs/guide/images/`), puis lance
  `python3 outils/generer-documentation.py` et commite `site/documentation/`.
- `python3 outils/generer-documentation.py --verifier` (dans `outils/verifier.sh`, la CI et
  `outils/publier-site.sh`) échoue si la doc générée est en retard.
- Une release n'est finie que quand le site est republié (workflow `deploy-site.yml`, ou
  `outils/publier-site.sh` depuis le Mac) : c'est lui qui met la doc en ligne.
- Rubrique Machines : un modèle ajouté au RIP (enum `Modele` de `serveur/src/machines.rs`) a
  sa page `docs/fr|en/machines/<modele>.md`, rangée sous sa famille dans les deux
  `sommaire.tsv` (Imprimantes, Plotters de découpe, Tables de découpe) ; le générateur échoue
  sinon. Détail dans `docs/guide/README.md`.
- Liens « Aide » câblés dans les logiciels : tableau de `docs/guide/README.md`. Ne pas
  renommer une page ou un titre cité sans les mettre à jour.
- Jamais de nom de RIP concurrent ni de détail d'hébergement interne dans la doc.

## Centre d'aide (help.suite-eko.com) : articles courts, brouillon par défaut

Les articles d'aide des catégories « E-KO RIP » et « E-KO RIP Cloud » du centre d'aide de la Suite
E-KO viennent de `docs/aide/` (format et style dans `docs/aide/README.md`). Ils complètent la
documentation d'e-ko.rip, qui reste la référence détaillée.

- **Règle de Mathieu (2026-11-04)** : 1 nouvelle fonction ou 1 nouveau module = un article détaillé
  dans `docs/aide/` (guide pas à pas, captures d'écran, réglages, cas particuliers), dans la même PR.
  Une fonction qui change = ses articles et ses captures mis à jour dans la même PR.
- Le centre d'aide remplace la documentation d'e-ko.rip : chaque page de `docs/guide/` et `docs/fr/`
  est couverte par des articles (`docs/aide/couverture/*.tsv`), puis redirigée en 301 vers eux. `python3 outils/publier-aide --verifier` (dans `outils/verifier.sh`) contrôle
  en-têtes, slugs préfixés `rip-` / `rip-cloud-`, images et liens.
- Envoi par le workflow `publier-aide.yml`, sur ordre de Mathieu seulement : en brouillon par
  défaut ; entrée `publier` seulement après « Publie l'aide ». Ne jamais toucher aux catégories
  maîtres, aux réglages du portail ni au reste du support. Jeton `CHATWOOT_DOC_TOKEN` (environnement
  `production`) : jamais dans le dépôt, un journal, une PR ou le chat.
- Après un envoi, commiter `docs/aide/televersements.json` (artefact du workflow).

## Feuille de route publique : on s'y tient (obligatoire)

La feuille de route publique est en ligne sur `https://e-ko.rip/feuille-de-route/`. Source :
`pages/feuille-de-route.md`, page générée par `python3 outils/generer-site.py`.

- Trois colonnes : « Fait », « En cours », « Planifié ». C'est l'engagement public du projet :
  on développe dans son ordre (« En cours » d'abord, puis « Planifié »). Toute fonction ajoutée
  ou abandonnée l'est d'abord dans ce fichier, avec l'accord de Mathieu.
- À chaque déploiement : faire avancer les points (Planifié → En cours → Fait) sans changer leur
  nom, mettre à jour `maj:`, régénérer et republier le site (comme la documentation).
- Ce qu'ont les RIP concurrents et pas encore E-KO RIP est « En cours » sur la feuille de route,
  et s'écrit « En cours… » dans les comparatifs du blog (jamais « non » ni « absent »).
- Jamais de nom de RIP concurrent ni de date promise dans la feuille de route. Les noms de
  concurrents ne sont permis que dans les comparatifs factuels et sourcés de `blog/`.

## Blog et pages du site

Articles du blog : `blog/fr/<adresse>.md` (format dans `blog/README.md`), générés dans
`site/blog/` par `python3 outils/generer-site.py`, qui pose aussi le menu et le pied communs sur
toutes les pages du site et écrit `sitemap.xml`. `--verifier` échoue si le site est en retard.
