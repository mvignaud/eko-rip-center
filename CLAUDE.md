# E-KO RIP Center — consignes pour Claude

## Release : publier sur le dépôt APT (obligatoire)

Chaque release se termine par la publication des paquets Linux sur le dépôt APT
`https://apt.e-ko.fr/rip/`. C'est par lui que les postes Linux et les serveurs RIP se mettent
à jour (`sudo apt update && sudo apt upgrade`). Une release qui n'est pas sur le dépôt n'arrive
chez personne.

Sur le serveur de test (`mathieu@eko-rip`, 192.168.10.109), une fois les `.deb` construits :

```bash
~/eko-depot/outils/publier-depot.sh \
  eko-rip-serveur_<version>-1_amd64.deb \
  e-ko-rip-center_<version>-1_amd64.deb
```

- **Les deux paquets à chaque release** : le serveur (`eko-rip-serveur`, via cargo-deb) et le
  client Linux natif (`e-ko-rip-center`, GTK 4, via cargo-deb dans `clients/linux/`, qui a
  remplacé le client Tauri de `client/` depuis la 0.29). La CI ne construit plus le client
  Linux, il se construit sur le serveur de test.
- Le script signe avec la clé GPG `depot@e-ko.fr` du trousseau du serveur de test et envoie par
  rsync au VPS OVH (`Host apt.e-ko.fr` dans `~/.ssh/config`, clé `~/.ssh/eko_depot`, compte
  `depot` limité à `/srv/depot`). Ne jamais régénérer cette clé : les postes refuseraient les
  mises à jour.
- `~/eko-depot/outils/` est une copie de `outils/publier-depot.sh` et `outils/installer-depot.sh`.
  Quand on les modifie dans le dépôt, il faut aussi recopier la nouvelle version là-bas.
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
- Une release n'est finie que quand le site est republié (`outils/publier-site.sh` depuis le
  Mac) : c'est lui qui met la doc en ligne.
- Rubrique Machines : un modèle ajouté au RIP (enum `Modele` de `serveur/src/machines.rs`) a
  sa page `docs/fr|en/machines/<modele>.md`, rangée sous sa famille dans les deux
  `sommaire.tsv` (Imprimantes, Plotters de découpe, Tables de découpe) ; le générateur échoue
  sinon. Détail dans `docs/guide/README.md`.
- Liens « Aide » câblés dans les logiciels : tableau de `docs/guide/README.md`. Ne pas
  renommer une page ou un titre cité sans les mettre à jour.
- Jamais de nom de RIP concurrent ni de détail d'hébergement interne dans la doc.

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
