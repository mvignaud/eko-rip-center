# Installer le client

Le client est le seul endroit où l'on travaille : envoi des fichiers, file d'attente,
réglages. Un navigateur ouvert sur `http://<serveur>:8640` ne montre qu'une page d'état
(version, machines, adresse à saisir dans le client) ; l'interface et l'API sont
réservées au client, reconnu à sa signature `EKO-RIP-Client/`.

Télécharger depuis la page des releases :

| Système | Fichier |
|---|---|
| macOS (Apple Silicon et Intel) | `E-KO.RIP.Center_<version>_universal.dmg` |
| Windows | `E-KO.RIP.Center_<version>_x64-setup.exe` ou `.msi` |
| Linux | `.deb` ou `.AppImage` |

## Premier lancement

Saisir l'adresse du serveur, par exemple `192.168.1.20:8640`. Le client la retient.
Pour en changer : menu **Serveur → Changer de serveur** (`Cmd/Ctrl + Maj + S`).

## Avertissement au premier lancement

Les paquets ne sont pas encore signés.
- **macOS** : clic droit sur l'application → **Ouvrir**, puis confirmer.
- **Windows** : « Windows a protégé votre ordinateur » → **Informations
  complémentaires** → **Exécuter quand même**.
