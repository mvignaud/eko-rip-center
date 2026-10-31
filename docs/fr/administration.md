# Administrer le serveur

Pour qui installe et fait vivre E-KO RIP Center à l'atelier. L'installation elle-même est
dans [installer le serveur](installation-serveur.md), l'usage dans [utilisation](utilisation.md).

## Au quotidien

```bash
systemctl status eko-rip-serveur
journalctl -u eko-rip-serveur -f
sudo systemctl restart eko-rip-serveur
```

Le journal dit chaque rendu raté, chaque sortie échouée (machine, travail, page et la
raison), et l'avertissement « aucun utilisateur déclaré » tant que l'accès est libre.
Système, dans le client, montre la version, la durée de conservation des travaux et la
place libre.

Un redémarrage coupe un rendu en cours : le travail passe en échec « interrompu », à
relancer avec **Relancer**. Les sorties en file restent dans la file. Les sessions sont
perdues : chacun se reconnecte.

## Où sont les données

Paquet : `/var/lib/eko-rip` (ailleurs : `EKO_RIP_DATA`).

| Chemin | Contenu |
|---|---|
| `machines.json` | catalogue des machines |
| `tarifs.json` | prix d'encre et de support, HT ou TTC |
| `reglages.json` | réglages enregistrés |
| `erp.json` | pont ERP : empreinte du jeton de l'ERP, secret HMAC, URL de l'ERP, activation, instance (droits 0600) — voir [pont-erp.md](pont-erp.md) |
| `cloud.json` | Cloud E-KO RIP : adresse de la plate-forme, site et clé reçus à l'appairage (droits 0600) — voir [cloud.md](cloud.md) |
| `pont/` | pont ERP : journal des événements (`journal.jsonl`) et état des envois (`etat.json`) ; ce n'est pas de la configuration, il n'est pas sauvegardé |
| `supports.json` | supports par machine et leurs modes d'impression (identifiants stables) ; repris de `tarifs.json` au premier démarrage |
| `charges.json` | support chargé dans chaque machine, saisi par l'opérateur ou lu sur l'Epson |
| `decoupe.json` | réglages de découpe du serveur (Système > Découpe : préfixes, calques non imprimables, tracés ouverts, tri, après coupe) ; compris dans la sauvegarde de la configuration et dans la copie avant mise à jour |
| `tons_directs.json` | table des tons directs (Système > Tons directs : nom exact, alias, couleur Lab) ; comprise dans la sauvegarde et dans la copie avant mise à jour |
| `tons_inconnus.json` | tons directs rencontrés absents de la table (nombre de travaux, dernière fois), listés dans Système > Tons directs |
| `utilisateurs.json` | comptes (mots de passe hachés, droits 0600) |
| `file.json` | file des sorties et historique |
| `profils/` | profils ICC déposés |
| `entrees/<réglage>/` | dossiers surveillés, un par réglage |
| `travaux/<id>/` | un dossier par travail : source, `job.json`, plaques (`prod/`), aperçus |
| `sorties/<machine>/` | plaques copiées pour une machine en connexion Dossier (Epson), fichiers de coupe d'une découpeuse en Dossier |
| `releves/<machine>.jsonl` | relevés SNMP (encres, rouleau) pour la consommation réelle |
| `sauvegardes/avant-<version>-<date>.tar.gz` | configuration copiée avant chaque mise à jour du paquet (machines, supports, réglages, tarifs, comptes, pont ERP, profils ; 10 gardées, droits 0600) |
| `*.json.prec` | version précédente de chaque fichier d'état, gardée à chaque écriture |
| `*.json.illisible` | copie d'un fichier d'état trouvé abîmé (coupure de courant) |

**Coupure de courant** : chaque fichier d'état est écrit sur le disque avant d'être mis en
place, et sa version précédente est gardée (`.prec`). Si un fichier est trouvé abîmé, le
serveur reprend la version précédente et le signale dans **Système** (« Fichiers d'état »).
Si les deux sont illisibles, le fichier abîmé est copié en `.illisible` et Système le dit :
restaurer une sauvegarde. Les comptes abîmés n'ouvrent jamais l'accès : la connexion reste
exigée.

Les travaux de plus de `EKO_RIP_CONSERVATION_JOURS` jours (30 par défaut) sont effacés
avec leurs sorties ; 0 les garde.

## Sauvegarder

- **La configuration** (machines, tarifs, réglages, profils) : Système > Sauvegarde de la
  configuration, dans le client. À refaire après chaque changement de réglages.
- **Tout, comptes compris** : copier `/var/lib/eko-rip`, service arrêté, ou au moins
  `utilisateurs.json` en plus de la sauvegarde de configuration. Les travaux sont
  volumineux (plaques pleine définition) et se refont depuis leurs sources.

Remonter sur un serveur neuf : installer, créer le compte administrateur, restaurer la
configuration depuis le client.

## Mettre à jour

Installer le paquet de la nouvelle version par-dessus l'ancien : les données restent.

```bash
sudo apt install ./eko-rip-serveur_<version>-1_amd64.deb
```

L'interface vient du serveur : les clients affichent les nouveautés dès que le serveur
est à jour. Mettre à jour un client ne sert que pour le client lui-même (fenêtre,
connexion au serveur).

## Publier sur le dépôt APT

Les postes Linux se mettent à jour depuis `https://apt.e-ko.fr/rip/` (VPS OVH, voir
eko-infra `ansible/depot.yml`). Après chaque release, sur le serveur de test (Linux) :

```bash
~/eko-depot/outils/publier-depot.sh "E-KO RIP Center_<version>_amd64.deb" eko-rip-serveur_<version>-1_amd64.deb
```

(`~/eko-depot/outils/` est une copie de `outils/` de ce dépôt.) Publier **les deux paquets à
chaque release**, le client et le serveur. Un serveur RIP qui a ajouté le dépôt avec le script
d'installation se met alors à jour, lui aussi, par `apt upgrade`.

Le script range les paquets dans `~/.eko-rip-depot/rip`, refait les index, les signe et
envoie le tout par rsync (compte `depot`, limité au dossier du dépôt). Il garde les cinq
dernières versions de chaque paquet (`EKO_DEPOT_GARDER`).

La clé de signature (`depot@e-ko.fr`) est créée au premier envoi dans le trousseau GPG de
cette machine, et seulement là. **La sauvegarder** (`gpg --export-secret-keys
depot@e-ko.fr`) : sans elle, les postes refusent toute mise à jour signée par une autre.

## Comptes

Voir [installer le serveur](installation-serveur.md#comptes). Mot de passe oublié :
redéclarer le compte (`utilisateur ajouter <nom> <rôle>`) en donne un nouveau. Plus
aucun administrateur : un compte déclaré sans rôle le devient.

## Machines

- **Kona (USB)** : la règle udev du paquet le déclare au pilote FTDI et crée `/dev/kona`
  pour le groupe `dialout` (celui du service). Vérifier : `ls -l /dev/kona` après
  l'avoir branché. Absent : débrancher, rebrancher, puis `journalctl -k | tail`.
- **Machine en réseau** (Graphtec, Roland) : l'adresse et le port doivent répondre depuis
  le serveur (`nc -vz <ip> <port>`).
- **Epson** : en connexion **Réseau (TCP)**, port 9100, le RIP imprime par son pilote
  direct (vérifier `nc -vz <ip> 9100` depuis le serveur) ; en **Dossier**, ses plaques
  vont dans `sorties/epson/`. La largeur du média, l'état et les alertes se lisent par
  SNMP si l'adresse est renseignée (le serveur parle SNMP lui-même, rien à installer) :
  sans SNMP, le pilote direct ne peut pas arrêter une impression sur une alerte.
- **IPP** : le flux de la machine (ESC/P2 de l'Epson, HP-GL d'une découpeuse) part par un
  Print-Job IPP, au format brut. Avec CUPS, créer une file **raw** (sans pilote) :
  `lpadmin -p epson -v socket://<ip>:9100 -E`, puis chemin `/printers/epson`, port 631.
  L'écran montre l'adresse complète (`ipp://hôte:631/printers/epson`). Vérifier :
  `lpstat -h <hôte> -p`. Le RIP attend la réponse de CUPS : une file arrêtée ou
  introuvable fait échouer la sortie avec le refus IPP en clair.
- **Renvoi vers un autre RIP E-KO** : le flux part à un autre serveur E-KO RIP Center, qui
  le passe tel quel à sa machine (son identifiant, par ex. `epson`), sans le remettre en
  file. Sur le serveur qui reçoit, créer le jeton (affiché une seule fois ; `retirer` ferme
  le renvoi, `etat` dit s'il est ouvert) :

  ```bash
  sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur renvoi jeton
  ```

  puis le coller dans la machine **Renvoi** du serveur qui envoie (hôte, port 8640). Le
  jeton n'est jamais réaffiché à l'écran ; il n'ouvre que la route de renvoi. Le flux
  passe en clair sur le réseau local : à garder entre serveurs de l'atelier.

## Pont ERP

Système > **Pont ERP** (administrateur) : URL de base donnée par l'ERP, secret HMAC
(`eko-rip-serveur erp secret`, ou collé ; jamais réaffiché), activation. Le RIP appelle
l'ERP en HTTPS sortant : aucun port à ouvrir. Si l'écran dit :

- **en erreur** : le code dit quoi faire (signature refusée → secret différent de celui de
  l'ERP ; signature expirée → horloge du serveur, `timedatectl` ; ERP injoignable → réseau,
  DNS, certificat). Rien n'est perdu : tout part au retour, dans l'ordre.
- **remplacé** : une autre installation parle à l'ERP à la place de celle-ci, ou la séquence
  a été remise à zéro depuis l'écran de l'ERP. Si c'est
  voulu, rien à faire ; sinon, **Tirer une nouvelle instance** (l'autre s'arrêtera à son tour).
- **anomalie** : l'ERP et le RIP ne comptent plus pareil ; voir avec l'ERP, puis tirer une
  nouvelle instance.

Détail : [pont-erp.md](pont-erp.md), § 6.

## Questions fréquentes

**Le navigateur n'affiche qu'une page d'état.** Voulu : l'interface ne vit que dans les
clients (Mac, PC, Linux). Une requête d'un navigateur à l'API reçoit « Réservé au client
E-KO RIP Center » (`client_requis`).

**« Accès libre » en rouge.** Aucun compte n'est déclaré : n'importe quel client du
réseau peut tout faire. Créer un compte administrateur.

**« Réservé à l'administrateur du RIP. »** Le compte est opérateur : machines, tarifs,
profils, réglages enregistrés et sauvegarde sont réservés à l'administrateur.

**« Trop d'essais pour cet utilisateur. »** 5 mots de passe faux : le nom est bloqué
5 minutes. Un redémarrage du service le débloque (et ferme toutes les sessions).

**Un rendu échoue.** Le message de Ghostscript est dans le détail de l'erreur et dans le
journal. Causes courantes : PDF protégé ou abîmé, délai `EKO_RIP_DELAI_S` dépassé sur un
très grand fichier, disque plein (Système). Les polices non incorporées et les images
trop peu définies sont signalées dès l'envoi par la pré-vérification.

**Très grands formats.** Une page dont le rendu dépasserait 3 Go (par exemple une bâche de
1,6 × 3 m à 720 dpi) est rendue par bandes horizontales, puis ses plaques sont recollées :
c'est plus long, mais rien n'est à régler. Une plaque recollée de plus de 3,5 Go est
écrite en BigTIFF. Prévoir la place : une plaque pèse jusqu'à largeur × hauteur en
points, soit près de 4 Go par encre pour cette bâche, avant compression.

**« Fichier trop volumineux pour le serveur. »** Au-delà de `EKO_RIP_MAX_MO` (8192 Mo par
défaut). Le monter dans `/etc/default/eko-rip-serveur`, puis redémarrer.

**« La sortie vers la machine a échoué. »** La raison est sous la sortie, dans la file :
port absent (`/dev/kona`), machine éteinte ou injoignable, droit refusé. Corriger, puis
**Refaire** depuis l'historique.

**« Une sortie attend encore cette machine dans la file. »** Une machine ne peut pas être
retirée du catalogue (ni par une restauration) tant qu'une sortie l'attend : la passer ou
l'annuler d'abord.

**« Envoi à l'imprimante impossible » sur l'Epson.** Un ton direct n'a pas d'encre sur
l'Epson (le renommer `White` ou `Metallic`, le convertir en CMJN, ou l'ajouter à la table
des tons directs avec sa couleur Lab, puis relancer le travail), ou la page porte
à la fois du blanc et de l'argent. En connexion Réseau (pilote direct), le blanc et
l'argent sont refusés pour l'instant : passer par une machine en Dossier.

**« L'imprimante signale une alerte : impression arrêtée. »** Le pilote direct a lu une
alerte bloquante par SNMP (capot ouvert…). La lever sur l'Epson, puis **Refaire** la
sortie depuis l'historique.

**« L'Epson ne répond pas ou ne donne pas la largeur du média. »** Vérifier l'adresse
SNMP de la machine, que l'Epson est allumée et sur le même réseau, et la communauté
(`EKO_RIP_EPSON_COMMUNAUTE`, `public` par défaut).

**La découpe est décalée par rapport à l'impression.** Faire une coupe d'essai, mesurer
l'écart, le saisir en **Décalage X / Y** de la machine (Système > Machines).

**La durée estimée ne correspond pas.** Mesurer la cadence réelle (m² par heure, sur la
laize) et la saisir en **Cadence** de la machine.
