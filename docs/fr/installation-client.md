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

L'écran de connexion cherche seul les serveurs E-KO RIP du réseau local et les propose dans
**Serveurs trouvés sur le réseau** (nom, adresse et version) : un clic suffit. **Rechercher à
nouveau** relance la recherche (environ 3 secondes). Si aucun serveur n'apparaît, saisir son
adresse sous **Saisir une adresse**, par exemple `192.168.1.20:8640`. Le client retient le
serveur choisi.
Pour en changer : menu **Serveur → Changer de serveur** (`Cmd/Ctrl + Maj + S`).

### Se connecter au cloud (Mac)

En tête de l'écran de connexion, **Serveur local** (ci-dessus) ou **Se connecter au cloud**. Le
second ouvre le navigateur sur la plate-forme E-KO (`https://platform.e-ko.rip`) pour autoriser
l'app avec son compte, puis liste les sites du compte (en ligne ou hors ligne, avec son rôle) :
un clic ouvre le site, sans mot de passe du RIP. Le client rouvre le dernier site au lancement
suivant ; **Serveur → Se déconnecter du cloud** (ou le bouton de l'écran) révoque l'accès. Le jeton
est gardé dans le trousseau du Mac (service `fr.e-ko.rip.cloud`). En essai, la clé
`plateformeCloud` (`defaults write fr.e-ko.rip-center plateformeCloud https://…`) vise une
autre plate-forme. **Administration…** est indisponible à distance pour
l'instant (page web qui ne peut pas porter le jeton). Détails : [cloud.md](cloud.md).

### Se connecter au cloud (Linux)

Le client Linux natif a le même écran : **Cloud → Se connecter au cloud** ouvre le navigateur par
défaut sur la plate-forme, qui revient sur une écoute locale temporaire (`127.0.0.1`, port libre,
5 minutes au plus ; **Annuler** l'abandonne). Si le navigateur ne s'ouvre pas, l'adresse
d'autorisation s'affiche pour la copier. Les sites du compte sont listés (un site hors ligne est
grisé) ; le dernier site ouvert est rouvert au lancement suivant. Le jeton est gardé dans
`~/.config/e-ko-rip-center/cloud.json` (droits 0600) ; **Se déconnecter du cloud** (écran des
sites, ou menu **Se déconnecter** sur un site cloud) le révoque et l'oublie. La variable
`EKO_RIP_CLOUD_URL` vise une autre plate-forme (essais). **Administration…** est indisponible à
distance pour l'instant.

La découverte utilise DNS-SD (mDNS, `_eko-rip._tcp`, port UDP 5353) : elle ne traverse pas les
routeurs ni les VPN (utiliser alors l'adresse). Sur Mac, macOS demande d'autoriser l'accès au
réseau local au premier lancement. Le serveur s'annonce sous le nom de la machine
(`EKO_RIP_CLOUD_NOM` s'il est réglé) ; `EKO_RIP_ANNONCE=0` désactive l'annonce.

## Avertissement au premier lancement

Les paquets ne sont pas encore signés (choix assumé pour l'instant : la signature Apple et
Microsoft viendra avec les versions abouties). Une autorisation suffit, une fois par poste :
- **macOS 15 et plus** : au refus, ouvrir **Réglages Système → Confidentialité et
  sécurité**, descendre jusqu'au message sur E-KO RIP Center → **Ouvrir quand même**, puis
  confirmer avec le mot de passe du Mac. Sur macOS plus ancien : clic droit sur
  l'application → **Ouvrir**, puis confirmer.
- **Windows** : « Windows a protégé votre ordinateur » → **Informations
  complémentaires** → **Exécuter quand même**.
