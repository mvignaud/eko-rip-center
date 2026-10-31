# Statistiques anonymes d'installation

Pour savoir combien d'E-KO RIP tournent (serveurs, postes Mac, Linux, Windows, téléphones),
chaque installation envoie un petit message à la plate-forme E-KO RIP
(`POST https://platform.e-ko.rip/api/ping`). Il part **à l'installation, à chaque mise à jour,
puis une fois par semaine**, que le serveur soit rattaché au cloud ou non.

## Ce qui est envoyé

| Champ | Exemple | |
|---|---|---|
| `id` | `3f9c…` (32 caractères) | tiré au hasard sur le poste, une fois ; ne dit rien du poste |
| `type` | `serveur`, `client-mac`, `client-linux`, `client-windows`, `client-mobile` | |
| `evenement` | `installation`, `mise_a_jour`, `hebdo` | |
| `version` | `0.30.0` | version du logiciel |
| `os` | `Ubuntu 26.04 LTS`, `macOS 26.1.0` | nom et version du système, rien d'autre |
| `machines`, `reglages` | `3`, `5` | serveur seulement : nombre de machines et de réglages |

Rien d'autre : ni nom de poste, d'atelier, de machine ou d'utilisateur, ni fichier, ni travail.
La plate-forme ne garde pas l'adresse IP. Réponse `204` ; en cas d'échec, nouvel essai plus tard,
sans rien afficher.

## Désactiver

- **Serveur** : décocher *Système › Statistiques anonymes*, ou `EKO_TELEMETRIE=0` dans
  `/etc/default/eko-rip-serveur` puis `systemctl restart eko-rip-serveur`.
- **Mac** : *Système › Confidentialité*. **Linux** : *Système › Ce poste*.
- Partout : variable d'environnement `EKO_TELEMETRIE=0`.

L'état est gardé dans `telemetrie.json` (dossier des données du serveur, dossier de
configuration du client Linux) ou dans les préférences du Mac (`telemetrie.*`).

## Côté plate-forme

Registre `installations.json` (une ligne par identifiant : type, OS, version, première et
dernière fois vu, machines, réglages), écrit au plus une fois par minute. Garde-fous : champs
bornés, un ping compté par identifiant et par heure, 2 000 nouveaux identifiants par heure et
200 000 au total au plus, corps limité à 4 Ko.

Statistiques (installations vues, actives depuis moins de 15 jours, par type, OS et version,
machines pilotées, nouvelles installations par semaine sur 8 semaines) :

- sur le VPS : `sudo -u eko-rip-cloud EKO_CLOUD_DATA=/var/lib/eko-rip-cloud eko-rip-cloud installations` ;
- en HTTP : `GET /api/installations` avec `Authorization: Bearer <EKO_CLOUD_JETON_STATS>`
  (route fermée tant que le jeton n'est pas réglé dans `/etc/default/eko-rip-cloud`). La page
  *Administration* du portail (super-admin) affiche aussi ces statistiques, carte « Installations ».
