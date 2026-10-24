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
| `utilisateurs.json` | comptes (mots de passe hachés, droits 0600) |
| `file.json` | file des sorties et historique |
| `profils/` | profils ICC déposés |
| `entrees/<réglage>/` | dossiers surveillés, un par réglage |
| `travaux/<id>/` | un dossier par travail : source, `job.json`, plaques (`prod/`), aperçus |
| `sorties/<machine>/` | plaques copiées pour une machine sans pilote (Epson) |

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
- **Epson** : pas encore de pilote ; ses plaques vont dans `sorties/epson/`. La largeur
  du média se lit par SNMP si l'adresse est renseignée (le serveur parle SNMP lui-même,
  rien à installer).

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

**« Fichier trop volumineux pour le serveur. »** Au-delà de `EKO_RIP_MAX_MO` (2048 Mo par
défaut). Le monter dans `/etc/default/eko-rip-serveur`, puis redémarrer.

**« La sortie vers la machine a échoué. »** La raison est sous la sortie, dans la file :
port absent (`/dev/kona`), machine éteinte ou injoignable, droit refusé. Corriger, puis
**Refaire** depuis l'historique.

**« Une sortie attend encore cette machine dans la file. »** Une machine ne peut pas être
retirée du catalogue (ni par une restauration) tant qu'une sortie l'attend : la passer ou
l'annuler d'abord.

**« Envoi à l'imprimante impossible » sur l'Epson.** Un ton direct n'a pas d'encre sur
l'Epson (le renommer `White` ou `Metallic`, ou le convertir en CMJN), ou la page porte
à la fois du blanc et de l'argent.

**« L'Epson ne répond pas ou ne donne pas la largeur du média. »** Vérifier l'adresse
SNMP de la machine, que l'Epson est allumée et sur le même réseau, et la communauté
(`EKO_RIP_EPSON_COMMUNAUTE`, `public` par défaut).

**La découpe est décalée par rapport à l'impression.** Faire une coupe d'essai, mesurer
l'écart, le saisir en **Décalage X / Y** de la machine (Système > Machines).

**La durée estimée ne correspond pas.** Mesurer la cadence réelle (m² par heure, sur la
laize) et la saisir en **Cadence** de la machine.
