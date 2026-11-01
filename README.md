# E-KO RIP Center

RIP d'impression grand format libre et gratuit ([AGPL-3.0](LICENSE)), site : [e-ko.rip](https://e-ko.rip).
Un serveur Linux reçoit les PDF des
postes par le client E-KO RIP Center (Mac, Linux, Windows) et pilote les machines qui lui
sont branchées ; un navigateur ne voit qu'une page d'état.

État : le PDF reçu est rendu en plaques d'encre (CMJN + une plaque par ton direct,
`White` et `Silver` compris) avec un aperçu dans l'écran, et ses tracés de découpe sont
convertis en HP-GL. Les découpeuses (Mutoh Kona, Graphtec) sont pilotées en direct ;
le pilote direct de l'Epson SC-S80600 (ESC/P2 par le réseau) est en essai.

Deux paquets : le **serveur** (Linux : `.deb`, Docker, binaire) et le **client** de
bureau (macOS, Windows, Linux), qui ouvre l'écran du serveur dans sa fenêtre.

**Image d'installation** (PC dédié, Ubuntu 24.04, ⚠️ efface le disque) : elle n'est plus
publiée sur GitHub. Elle se construit en local (`image/construire-iso.sh`) et se dépose sur
le nuage d'E-KO, un dossier par version (`EKO RIP/vX.Y.Z/`), avec un lien public en lecture
seule ; le lien de chaque version est dans
[l'installation du serveur](docs/fr/installation-serveur.md#0-pc-dédié--image-dinstallation-ubuntu).
Dernière image : 0.16.0.

Documentation utilisateur en ligne : [e-ko.rip/documentation](https://e-ko.rip/documentation/)
([English](https://e-ko.rip/documentation/en/)), source dans [docs/guide/](docs/guide/).
Références : [français](docs/fr/) · [English](docs/en/) · machines
([fr](docs/fr/machines.md), [en](docs/en/machines.md)) · glossaire ([fr](docs/fr/glossaire.md),
[en](docs/en/glossary.md)) ·
[changelog](CHANGELOG.md). Administration et questions fréquentes :
[administration](docs/fr/administration.md) · [running the server](docs/en/administration.md).

## Développement

### Prérequis

- Rust stable
- Ghostscript (`gs` ; il est lancé en `-dSAFER`)

### Lancer le serveur

```bash
cargo run --release -p eko-rip-serveur
```

Puis ouvrir `http://<serveur>:8640` depuis un poste.

| Variable | Défaut | Rôle |
|---|---|---|
| `EKO_RIP_ADDR` | `0.0.0.0:8640` | adresse d'écoute |
| `EKO_RIP_DATA` | `eko-rip-data` | dossier des travaux |
| `EKO_RIP_GS` | `gs` | commande Ghostscript |
| `EKO_RIP_MAX_MO` | `8192` | taille maximale d'un fichier reçu |
| `EKO_RIP_DELAI_S` | `1800` | durée maximale d'un rendu |

Tant qu'aucun compte n'est déclaré, l'accès à l'écran reste libre (l'écran l'affiche en
rouge) : déclarer au moins un compte avec `eko-rip-serveur utilisateur ajouter <nom>`
pour le fermer (voir [les comptes](docs/fr/installation-serveur.md#comptes)). Les
dossiers surveillés ne demandent pas de compte, et les routes `/api/erp/` n'acceptent
que le jeton de l'ERP. Dans tous les cas, le serveur ne doit être joignable que depuis
le réseau de l'atelier.

### Client

```bash
cd client && cargo tauri dev
```

### Tests

```bash
cargo test
```

Le test de rendu demande Ghostscript.

La CI (`.github/workflows/ci.yml`) ne se lance plus qu'à la main, faute de minutes GitHub
Actions. Avant de pousser, lancer `./outils/verifier.sh` : il refait ses contrôles (syntaxe
des pages, doublons de traduction, clippy, tests du serveur), sans construire le `.deb`.

## Licence

E-KO RIP Center est un logiciel libre, publié sous la
[GNU Affero General Public License v3.0 ou ultérieure](LICENSE) (AGPL-3.0-or-later).
Chacun peut l'utiliser, l'étudier, le modifier et le redistribuer. Une version modifiée
qui est distribuée, ou proposée à d'autres par le réseau (comme le Cloud E-KO RIP), doit
publier ses sources sous la même licence.

Ghostscript, que le serveur lance comme programme séparé, garde sa propre licence
(AGPL-3.0, Artifex). Les noms et logos E-KO ne sont pas couverts par la licence du code :
voir [TRADEMARKS.md](TRADEMARKS.md). Pour contribuer : [CONTRIBUTING.md](CONTRIBUTING.md).
