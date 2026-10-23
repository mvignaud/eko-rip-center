# E-KO RIP Center

RIP d'impression grand format, usage interne. Un serveur Linux reçoit les PDF des
postes (Mac, Linux, Windows) par le navigateur et pilote les machines qui lui sont
branchées.

État : phase P0. Le PDF reçu est rendu en plaques d'encre (CMJN + une plaque par ton
direct, `White` et `Silver` compris) avec un aperçu dans l'écran. Aucun pilote de
machine pour l'instant.

Deux paquets : le **serveur** (Linux : `.deb`, Docker, binaire) et le **client** de
bureau (macOS, Windows, Linux), qui ouvre l'écran du serveur dans sa fenêtre.

Documentation : [français](docs/fr/) · [English](docs/en/) ·
[changelog](CHANGELOG.md).

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
| `EKO_RIP_MAX_MO` | `2048` | taille maximale d'un PDF reçu |
| `EKO_RIP_DELAI_S` | `1800` | durée maximale d'un rendu |

Le serveur n'a pas d'authentification : il ne doit être joignable que depuis le
réseau de l'atelier.

### Client

```bash
cd client && cargo tauri dev
```

### Tests

```bash
cargo test
```

Le test de rendu demande Ghostscript.
