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
