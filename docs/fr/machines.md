# Aide par machine

Une fiche par modèle du catalogue (Système > Machines) : ce que le RIP en fait, comment la
brancher, quoi régler sur la machine et dans le RIP, comment vérifier, et les ennuis
courants. Le détail des fonctions est dans [l'utilisation](utilisation.md), l'exploitation
du serveur dans [l'administration](administration.md), et les mots dans le
[glossaire](glossaire.md).

Pour toutes les machines :

- **Identifiant** : minuscules, chiffres et `-` (`epson`, `kona`…). Il nomme le dossier
  `sorties/<machine>/` et sert au renvoi d'un autre RIP : ne pas le changer à la légère.
- **Une sortie à la fois**, dans l'ordre de la file de la machine. Une machine qui a encore
  une sortie à passer ne peut pas être retirée du catalogue.
- **Adresse IP fixe** pour toute machine en réseau (voir les
  [exigences du poste](installation-serveur.md#exigences-du-poste-serveur)).

## Epson SureColor SC-S80600 (impression)

**Ce que fait le RIP** : il rend les plaques, les sépare sur les 9 encres (couples clair et
foncé, limite d'encre), les trame et envoie l'impression lui-même (pilote direct, ESC/P2).

**Brancher** : câble réseau, IP fixe.

| Connexion | Effet |
|---|---|
| **Réseau (TCP)**, hôte = IP de l'Epson, port **9100** | Pilote direct : le RIP imprime. |
| **IPP** vers une file CUPS **raw** | Même flux, passé par CUPS. |
| **Renvoi** vers un autre RIP E-KO | Même flux, passé à l'Epson de l'autre serveur. |
| **Dossier** | Rien n'est imprimé : un fichier par canal (`K.tif`… `WH.tif`) et `envoi.json` dans `sorties/<machine>/`. |

**Dans le RIP** : **SNMP (largeur du média)** = IP de l'Epson (largeur et n° du support chargé,
encres, alertes), aussi dans `EKO_RIP_EPSON_SNMP` pour l'écran du média chargé ; **Origine
X / Y impression** à régler sur un essai papier ; **Cadence (m²/h)** mesurée à l'atelier ;
catalogue des supports et de leurs modes (Système > Supports de l'Epson), avec le **n° de
support** enregistré dans l'imprimante.

**Sur l'Epson** : enregistrer chaque support sous un n° et reporter ce n° dans le catalogue
du RIP ; c'est lui qui fait reconnaître le support chargé.

**Vérifier** : `nc -vz <ip> 9100` depuis le serveur ; Système > Média chargé dans l'Epson
doit montrer la laize, le rouleau restant et les encres.

**Ennuis courants**

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

Voir : [pilote direct de l'Epson](utilisation.md#pilote-direct-de-lepson),
[supports et modes](utilisation.md#supports-et-modes-dimpression),
[média chargé](utilisation.md#média-chargé-dans-lepson),
[encre et coût](utilisation.md#encre-et-coût).

## Mutoh Kona 1650 (découpe)

**Ce que fait le RIP** : il envoie la découpe comme Mutoh CutServer (HP-GL au pas de
0,01 mm), avec lecture du cadre et du code-barres pour le repérage.

**Brancher** : USB (adaptateur série FTDI). La règle udev du paquet crée `/dev/kona` pour le
groupe `dialout`. Connexion **Série / USB-série**, port `/dev/kona`, réglages
`9600 cs8 -cstopb -parenb crtscts raw -echo` (9600 bauds, 8N1, RTS/CTS).

**Sur le Kona** : résolution **0,010 mm**.

**Dans le RIP** : pas de découpe **0,01 mm** ; **réglages de coupe** (vitesse, force,
accélération, outil, motif), le premier servant par défaut ; **Décalage X / Y** mesuré sur
une coupe d'essai ; **Découpe retournée** si la coupe sort tournée de 180°.

**Repérage** : à l'envoi, **Repérage Kona** ; le **cadre + code-barres (type 2)** est celui
éprouvé à l'atelier. Engager dans le Kona le bord que montre la flèche ▲ du cadre.

**Vérifier** : `ls -l /dev/kona` après l'avoir branché.

**Ennuis courants**

- *`/dev/kona` absent* : débrancher, rebrancher, puis `journalctl -k | tail`.
- *Arrêt sur « EPOS 6213 »* : cadre type 1 sans code-barres envoyé en direct ; le Kona ne
  le lit pas seul. Prendre le type 2.
- *Découpe décalée* : coupe d'essai, mesurer, saisir le **Décalage X / Y**.

Voir : [repérage pour le Kona](utilisation.md#repérage-pour-le-kona),
[découpe](utilisation.md#découpe), [variantes](utilisation.md#variantes-de-machine).

## Graphtec FC7000 (découpe)

**Ce que fait le RIP** : il envoie la découpe en HP-GL, au pas de 0,025 mm.

**Brancher** : réseau (IP fixe) en connexion **Réseau (TCP)**, ou série / USB-série.

**Sur la Graphtec** : commandes en **HP-GL** (Menu > Interface > Command) ; le repérage
**ARMS** se règle au panneau de la machine.

**Dans le RIP** : pas de **0,025 mm** ; réglages de coupe ; **Décalage X / Y**.

**Vérifier** : `nc -vz <ip> <port>` depuis le serveur.

**Ennuis courants** : *rien ne coupe* → l'interface n'est pas en HP-GL ; *sortie en échec,
machine injoignable* → adresse, port, câble.

Voir : [découpe](utilisation.md#découpe), [machines](utilisation.md#machines).

## Roland VersaCAMM VP-300 / VP-540 (découpe)

**Ce que fait le RIP** : la découpe part par le réseau (laize 762 ou 1371 mm). L'impression
par la Roland n'existe pas encore : elle viendra après une capture réseau. Le tout reste à
éprouver à l'atelier.

**Brancher** : réseau, IP fixe, connexion **Réseau (TCP)**.

**Vérifier** : `nc -vz <ip> <port>` depuis le serveur.

## Connexions

| Connexion | Pour | À saisir | Vérifier |
|---|---|---|---|
| **Réseau (TCP)** | Epson (pilote direct), Graphtec, Roland | hôte (IP fixe), port (9100 pour l'Epson) | `nc -vz <ip> <port>` |
| **Série / USB-série** | Kona, découpeuse en USB-série | port (`/dev/kona`), réglages `stty` | `ls -l <port>` ; le service est dans `dialout` |
| **Périphérique (USB…)** | machine en USB imprimante | chemin (`/dev/usb/lp0`…) | `ls -l <chemin>` |
| **Dossier** | pas de pilote, logiciel tiers qui surveille le dossier | rien | `ls /var/lib/eko-rip/sorties/<machine>/` |
| **IPP** | file CUPS **raw**, imprimante IPP | hôte, port 631, chemin (`/printers/<file>`) | `lpstat -h <hôte> -p` |
| **Renvoi vers un autre RIP E-KO** | machine branchée sur un autre RIP E-KO | hôte, port 8640, identifiant de la machine là-bas, jeton | `eko-rip-serveur renvoi etat` sur l'autre serveur |

Détail des connexions IPP et Renvoi : [administration, machines](administration.md#machines).
