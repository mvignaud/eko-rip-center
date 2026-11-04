---
titre: Modifier les réglages avancés du serveur
description: Changez le port, le dossier des données, la durée de conservation, les seuils d'espace libre et les autres réglages du serveur dans un fichier.
sous_rubrique: rip-administration
position: 21
mots_cles: réglages avancés, variables, eko_rip, port, conservation, seuil, snmp, plotter, annonce, fichier default
---

Les réglages avancés du serveur se donnent par des variables dans un seul fichier. Cet article explique comment les modifier et donne la liste complète, avec leur valeur par défaut.

## Avant de commencer

Vous avez un terminal sur le serveur et un accès `sudo`. Un redémarrage du service coupe le rendu en cours : choisissez un moment calme.

## Étapes

1. Ouvrez le fichier des réglages : `sudo nano /etc/default/eko-rip-serveur`.
2. Ajoutez ou modifiez une ligne par réglage, de la forme `EKO_RIP_CONSERVATION_JOURS=14`.
3. Enregistrez, puis redémarrez le service : `sudo systemctl restart eko-rip-serveur`.
4. Vérifiez dans la vue **Système** du client que la valeur est prise en compte (durée de conservation, espace libre).

### Réglages courants

| Réglage | Par défaut | Rôle |
|---|---|---|
| `EKO_RIP_ADDR` | toutes les interfaces, port `8640` | adresse et port d'écoute |
| `EKO_RIP_DATA` | `eko-rip-data` (paquet : `/var/lib/eko-rip`) | dossier des données et des travaux |
| `EKO_RIP_GS` | `gs` | commande Ghostscript |
| `EKO_RIP_MAX_MO` | `8192` | taille maximale d'un fichier reçu (PDF, SVG, TIFF, PNG, JPEG), en Mo |
| `EKO_RIP_DELAI_S` | `1800` | durée maximale d'un rendu, en secondes |
| `EKO_RIP_RENDUS` | un par cœur, 4 au plus | rendus menés en même temps (1 à 16) ; chacun prend un cœur et de la mémoire, baissez sur un petit poste |
| `EKO_RIP_CONSERVATION_JOURS` | `30` | travaux effacés après ce nombre de jours (0 : jamais) |
| `EKO_RIP_SEUIL_LIBRE_MO` | `5120` | espace libre à garder, en Mo : en dessous, les plus vieux travaux inactifs sont purgés et un travail qui ne tiendrait pas est refusé (motif `espace_insuffisant`) ; un dépôt en dossier surveillé devient un travail en erreur avec ce motif ; 0 : aucun seuil |
| `EKO_RIP_SEUIL_ALERTE_MO` | `20480` | seuil bas, en Mo : en dessous, l'alerte « disque presque plein » (`espace_disque_bas`) s'ajoute à chaque nouveau travail et l'API signale `disque_niveau: bas` ; 0 : aucune |
| `EKO_RIP_PURGER_ERP_FAITS` | `0` | `1` : un travail venu de l'ERP est effacé dès qu'il est fait (une sortie terminée, aucune à passer ni en échec), sans attendre la conservation |
| `EKO_RIP_PLOTTER` | `/dev/kona` | port de la découpeuse en USB |
| `EKO_RIP_PLOTTER_STTY` | `9600 cs8 -cstopb -parenb crtscts raw -echo` | réglages du port de la découpeuse |
| `EKO_RIP_EPSON_SNMP` | (vide) | adresse de l'Epson : largeur du média chargé lue par SNMP |
| `EKO_RIP_EPSON_COMMUNAUTE` | `public` | communauté SNMP de l'Epson |
| `EKO_RIP_EXPO_JETON` | (vide) | jeton d'accès du service de notifications de l'application mobile, si la « sécurité renforcée des push » est activée sur son projet |
| `EKO_RIP_ANNONCE` | `1` | `0` : ne pas s'annoncer sur le réseau local (les clients trouvent le serveur seuls, sinon l'adresse est à saisir) |
| `EKO_RIP_CLOUD_NOM` | nom du PC | nom proposé pour le site sur le Cloud |
| `EKO_RIP_LENTEUR_MIN` | `30` | durée, en minutes, au-delà de laquelle un rendu est signalé comme lent dans les rapports automatiques |
| `EKO_TELEMETRIE` | actif | `0` : n'envoie plus les statistiques anonymes d'installation |
| `EKO_RIP_ACCES_NAVIGATEUR` | `0` | `1` : interface complète aussi dans un navigateur (développement seulement) |

## Bon à savoir

- Le paquet crée ce fichier ; les modifications survivent aux mises à jour.
- Le délai d'un rendu (`EKO_RIP_DELAI_S`) dit vite si le poste est trop juste : un rendu qui dépasse 30 minutes échoue.
- La taille maximale d'un fichier s'applique à l'envoi : au-delà, le client affiche « Fichier trop volumineux pour le serveur. » Relevez `EKO_RIP_MAX_MO` puis redémarrez.
- Sous 5 Go libres par défaut, le serveur efface lui-même les plus vieux travaux inactifs : augmentez le seuil si vous voulez plus de marge.
- Pour savoir où chaque fichier de données est rangé, voir [Savoir où sont les données du serveur](rip-savoir-ou-sont-les-donnees-du-serveur.md).

## Voir aussi

- [Surveiller le service du serveur](rip-surveiller-le-service-du-serveur.md)
- [Choisir le poste serveur](../rip-premiers-pas/rip-choisir-le-poste-serveur.md)
- [Résoudre un rendu en échec ou un fichier refusé](../rip-depannage/rip-un-rendu-echoue.md)
- [Comprendre les statistiques anonymes](rip-comprendre-les-statistiques-anonymes.md)
