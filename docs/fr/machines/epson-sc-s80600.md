# Epson SureColor SC-S80600

## Ce que fait le RIP

Il rend les plaques, les sépare sur les 9 encres (couples clair et
foncé, limite d'encre), les trame et envoie l'impression lui-même (pilote direct, ESC/P2).

## Brancher

Câble réseau, IP fixe.

| Connexion | Effet |
|---|---|
| **Réseau (TCP)**, hôte = IP de l'Epson, port **9100** | Pilote direct : le RIP imprime. |
| **IPP** vers une file CUPS **raw** | Même flux, passé par CUPS. |
| **Renvoi** vers un autre RIP E-KO | Même flux, passé à l'Epson de l'autre serveur. |
| **Dossier** | Rien n'est imprimé : un fichier par canal (`K.tif`… `WH.tif`) et `envoi.json` dans `sorties/<machine>/`. |

## Dans le RIP

**SNMP (largeur du média)** = IP de l'Epson (largeur et n° du support chargé,
encres, alertes), aussi dans `EKO_RIP_EPSON_SNMP` pour l'écran du média chargé ; **Origine
X / Y impression** à régler sur un essai papier ; **Cadence (m²/h)** mesurée à l'atelier ;
catalogue des supports et de leurs modes (Système > Supports de l'Epson), avec le **n° de
support** enregistré dans l'imprimante.

## Sur l'Epson

Enregistrer chaque support sous un n° et reporter ce n° dans le catalogue
du RIP ; c'est lui qui fait reconnaître le support chargé.

## Vérifier

`nc -vz <ip> 9100` depuis le serveur ; Système > Média chargé dans l'Epson
doit montrer la laize, le rouleau restant et les encres.

## Ennuis courants

- *« Envoi à l'imprimante impossible »* : un ton direct n'a pas d'encre sur l'Epson (le
  renommer `White` ou `Metallic`, le convertir en CMJN, ou l'ajouter à la table des tons
  directs), ou la page porte blanc et argent à la fois. Le blanc et l'argent ne passent
  pas encore par le pilote direct : machine en **Dossier** en attendant.
- *« L'imprimante signale une alerte : impression arrêtée »* : alerte bloquante lue par
  SNMP (capot ouvert…). La lever, puis **Refaire**.
- *Sortie en attente « Autre support »* : le support chargé n'est pas celui du travail ;
  charger le bon, ou choisir le support chargé à la main.
- *Pas de largeur de média* : adresse SNMP, Epson allumée et sur le même réseau,
  communauté (`EKO_RIP_EPSON_COMMUNAUTE`, `public` par défaut).

Voir : [pilote direct de l'Epson](../utilisation.md#pilote-direct-de-lepson),
[supports et modes](../utilisation.md#supports-et-modes-dimpression),
[média chargé](../utilisation.md#média-chargé-dans-lepson),
[encre et coût](../utilisation.md#encre-et-coût).
