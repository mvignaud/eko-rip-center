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
| Linux | dépôt APT (ci-dessous), ou `.deb` / `.AppImage` |

## Linux : par le dépôt APT (mises à jour par `apt`)

Sur Ubuntu ou Debian, une seule fois :

```bash
wget -qO- https://apt.e-ko.fr/rip/installer | sudo sh
```

Le script ajoute la clé du dépôt et `/etc/apt/sources.list.d/eko-rip.list`, puis installe
le client. Ensuite, le client se met à jour avec le reste du système :

```bash
sudo apt update && sudo apt upgrade
```

## Premier lancement

Saisir l'adresse du serveur, par exemple `192.168.1.20:8640`. Le client la retient.
Pour en changer : menu **Serveur → Changer de serveur** (`Cmd/Ctrl + Maj + S`).

## Avertissement au premier lancement

Les paquets ne sont pas encore signés (choix assumé pour l'instant : la signature Apple et
Microsoft viendra avec les versions abouties). Une autorisation suffit, une fois par poste :
- **macOS 15 et plus** : au refus, ouvrir **Réglages Système → Confidentialité et
  sécurité**, descendre jusqu'au message sur E-KO RIP Center → **Ouvrir quand même**, puis
  confirmer avec le mot de passe du Mac. Sur macOS plus ancien : clic droit sur
  l'application → **Ouvrir**, puis confirmer.
- **Windows** : « Windows a protégé votre ordinateur » → **Informations
  complémentaires** → **Exécuter quand même**.
