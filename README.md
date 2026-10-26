# E-KO RIP Center

RIP d'impression grand format, usage interne. Un serveur Linux reçoit les PDF des
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

Documentation : [français](docs/fr/) · [English](docs/en/) ·
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
