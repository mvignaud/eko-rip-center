---
titre: Consulter le glossaire du RIP
description: Les mots de l'écran et de l'aide (travail, plaque, laize, BAT, repérage, pont ERP…) avec leur définition et leur équivalent anglais.
sous_rubrique: rip-premiers-pas
position: 25
mots_cles: glossaire, définitions, vocabulaire, plaque, laize, bat, repérage, ton direct, anglais
---

Ce glossaire explique les mots employés à l'écran et dans l'aide, avec leur équivalent anglais, celui de l'interface en anglais. Il sert de référence : parcourez-le quand un terme vous est inconnu.

## Avant de commencer

Aucun prérequis. Les définitions sont groupées en cinq familles : travaux et file, encre et couleur, support et mise en page, découpe, machines et serveur.

## Étapes

1. Repérez la famille du mot cherché (travail, couleur, support, découpe, machine).
2. Cherchez le mot en gras dans le tableau de cette famille. Une colonne donne son équivalent anglais, utile si vous changez la langue de l'interface.
3. Pour la procédure qui utilise le mot, suivez les liens de la section **Voir aussi**.

### Travaux et file

| Terme | En anglais | Définition |
|---|---|---|
| **Travail** | Job | Un fichier reçu (PDF, SVG, TIFF, PNG, JPEG) avec sa demande : réglages, pages, plaques rendues, fiche. |
| **Rendu** | Rendering | Passage du fichier en plaques d'encre (Ghostscript), fait une fois à la réception ; une impression ou une découpe repart sans refaire le rendu. |
| **Pré-vérification** | Preflight | Contrôle fait à la réception : polices non incorporées, images trop peu définies, tracés de découpe, formats. Elle signale, ou refuse si le réglage le demande. |
| **Réglage** | Preset | Jeu de choix enregistré sous un nom (résolution, support, blanc, découpe, repérage…), appliqué à l'envoi ou par un dossier surveillé. |
| **Dossier surveillé** | Hot folder | Dossier du serveur dont chaque fichier déposé devient un travail, avec le réglage du dossier (`entrees/<réglage>/`). |
| **Règle** | Rule | Dans `entrees-auto/`, choisit le réglage d'un fichier d'après son nom ou le format de sa page. |
| **Motif à balises** | Tagged name pattern | Dans `entrees-auto/`, forme du nom de fichier (`{nom}__{machine}__{reglage}__{copies}`) qui donne réglage, machine, copies… |
| **Fiche travail** | Job ticket | N° de commande, client, société, quantité, échéance, notes et opérateur d'un travail. |
| **Sortie** | Output | Une page d'un travail envoyée à une machine (impression ou découpe), dans la file d'attente. |
| **File d'attente** | Queue | Les sorties à passer, machine par machine, dans l'ordre ; une machine passe une sortie à la fois. |
| **Raison d'attente** | Waiting reason | Pourquoi une sortie ne part pas (machine injoignable, autre support, trop large…) ; les suivantes passent devant. |
| **Tampon** | Spool | Poids des fichiers que les sorties à passer enverront aux machines. |
| **Historique** | History | Les sorties finies, en échec ou annulées ; **Refaire** relance sans nouveau rendu. |
| **BAT** (bon à tirer) | Proof | Épreuve à valider ; sur une 2e machine, la copie « -BAT » part et l'original attend, suspendu. |
| **Pool** | Pool | Machines identiques regroupées sous un nom : la sortie part sur la première prête. |
| **Purge** | Purge | Effacement des vieux travaux inactifs, après la durée de conservation ou sous le seuil d'espace libre. |

### Encre et couleur

| Terme | En anglais | Définition |
|---|---|---|
| **Plaque** | Plate | Image 8 bits d'une encre (0 = pleine encre, 255 = pas d'encre). |
| **Ton direct** | Spot colour | Couleur nommée du PDF (`White`, `Silver`, `CutContour`…), sortie sur sa propre plaque au lieu d'être mélangée au CMJN. |
| **Ton de découpe** | Cut spot colour | Ton direct qui porte les tracés de découpe (`CutContour`…) ; jamais imprimé. |
| **Table des tons directs** | Spot colour table | Nom exact, alias et couleur Lab de chaque ton direct, pour le convertir ou le reconnaître. |
| **Blanc** / **Argent** | White / Silver (metallic) | Encres spéciales de l'Epson, reconnues par le nom du ton direct (`White`, `Metallic`…). |
| **Sous-couche** | Underlay | Blanc posé sous l'encre, pour un support transparent ou coloré. |
| **Rétraction** | Choke | Recul du blanc depuis le bord de l'encre, pour ne pas dépasser de la couleur. |
| **Profil ICC** | ICC profile | Description de la couleur d'un support ou d'un espace, déposée sur le serveur. |
| **Intention de rendu** | Rendering intent | Façon de ramener une couleur hors gamut dans celle du support. |
| **Mode d'impression** | Print mode | Résolution, passes, chauffage, bascule des claires et encre totale d'un support. |
| **Bascule des claires** | Light-ink switch | Où l'encre foncée prend le relais de la claire (Lc, Lm, Lk) : douce, équilibrée, franche. |
| **Encre totale** / **Limite d'encre** | Total ink limit | Somme maximale des encres en un point, en %. |
| **Linéarisation** | Linearisation | Mesure de plages d'encre pour régler la réponse de chaque encre sur un support. |
| **Tramage** | Screening | Passage des valeurs d'encre en points (4 tailles de goutte pour l'Epson). |
| **Relevé** | Reading | Niveaux d'encre et rouleau lus sur l'imprimante, pour la consommation réelle. |

### Support et mise en page

| Terme | En anglais | Définition |
|---|---|---|
| **Support** | Media | Matière imprimée (vinyle, bâche…), avec sa laize, ses encres et ses modes. |
| **Support chargé** | Loaded media | Support présent dans la machine, lu sur l'Epson ou choisi par l'opérateur. |
| **Laize** | Roll width | Largeur utile du rouleau. |
| **Fond perdu** | Bleed | Image prolongée au-delà du format fini, pour qu'une coupe un peu décalée ne laisse pas de blanc. |
| **Œillet** | Grommet | Repère d'œillet posé sur le pourtour d'une bâche. |
| **Mosaïque** | Tiling | Grand visuel découpé en morceaux qui tiennent sur la laize. |
| **Lé** | Panel | Bande verticale d'une mosaïque, au plus large comme la laize. |
| **Tuile** | Tile | Un morceau d'une mosaïque en colonnes × rangées ; chaque tuile est une page. |
| **Recouvrement** | Overlap | Partie commune à deux lés voisins, pour les raccorder à la pose. |
| **Planche** | Sheet | Plusieurs travaux posés côte à côte sur une même page. |
| **Imbrication** | Nesting | Pose des travaux sur une planche, à la main ou d'elle-même dans la file. |

### Découpe

| Terme | En anglais | Définition |
|---|---|---|
| **Tracé** | Path | Ligne que la découpeuse suit. |
| **Couche de découpe** | Cut layer | Les tracés d'un même nom de ton, coupés ensemble avec un même réglage de coupe. |
| **Réglage de coupe** | Cut setting | Vitesse, force, accélération, outil et motif d'une couche. |
| **Repérage** | Registration | Lecture par la découpeuse de repères imprimés, pour couper à la place exacte de l'impression. |
| **Repères** / **Cadre** | Marks / Frame | Croix ou cadre (avec code-barres pour le Kona) imprimés autour de l'image pour le repérage. |
| **Détourage automatique** | Auto contour | Tracé de découpe calculé autour de l'image. |
| **Tolérance de découpe** | Cutting tolerance | Encre prolongée au-delà des contours de coupe, pour qu'une coupe un peu décalée ne laisse pas de liseré blanc. |
| **Compensation de cintrage** | Bowing compensation | La coupe suit l'impression sortie courbée des sécheurs (milieu en retard sur les bords). |
| **Décalage X / Y** | Offset X / Y | Écart mesuré entre la découpe et l'impression, ajouté à chaque découpe d'une machine. |
| **Découpe retournée** | Cut turned (180°) | Découpe tournée de 180°, quand le support entre par l'autre bout. |
| **Variante** | Variant | Copie d'une découpeuse avec son propre jeu de repères. |

### Machines et serveur

| Terme | En anglais | Définition |
|---|---|---|
| **Machine** | Machine | Imprimante ou découpeuse du catalogue (Système > Machines). |
| **Connexion** | Connection | Comment le serveur joint la machine : réseau TCP, série, périphérique, dossier, IPP ou renvoi. |
| **Pilote direct** | Direct driver | Le RIP fabrique lui-même le flux de l'imprimante (ESC/P2 de l'Epson) et l'envoie. |
| **Connexion Dossier** | Folder connection | Rien n'est envoyé : la sortie est copiée dans `sorties/<machine>/`. |
| **IPP** | IPP | Protocole d'impression réseau (CUPS) ; le flux part en brut par un Print-Job. |
| **Renvoi** | Forwarding | Le flux part à un autre E-KO RIP Center, qui le passe à sa machine. |
| **Jeton** | Token | Secret qui ouvre un accès machine à machine (renvoi, ERP) ; jamais réaffiché. |
| **Cadence** | Speed | Surface imprimée par heure (m²/h), pour estimer les durées. |
| **SNMP** | SNMP | Lecture de l'état de l'Epson par le réseau (média, encres, alertes). |
| **Pont ERP** | ERP bridge | Échanges signés entre le RIP et l'ERP E-KO (contrat `eko-rip-erp/1`). |
| **Seuil d'espace libre** | Free space threshold | Place à garder sur le disque des données ; en dessous, purge et refus des gros travaux. |
| **Client** | Client | Application de bureau (Mac, Windows, Linux) qui ouvre l'écran du serveur. |

## Bon à savoir

- Dans l'interface, le menu de langue passe de français à anglais (bouton **English** de l'interface d'administration) : les mots de la colonne « En anglais » sont ceux affichés alors.
- Une plaque est l'image 8 bits d'une encre : 0 pour la pleine encre, 255 pour aucune encre.
- Un ton de découpe (`CutContour`…) n'est jamais imprimé : il porte seulement les tracés de coupe.
- Le **Pont ERP** est l'échange signé entre le RIP et l'ERP E-KO : voir [Comprendre le pont ERP](../rip-erp/rip-comprendre-le-pont-erp.md).

## Voir aussi

- [Envoyer un travail](../rip-travaux/rip-envoyer-un-travail.md)
- [Déclarer les tons directs](../rip-couleur/rip-declarer-les-tons-directs.md)
- [Régler un support et ses modes](../rip-couleur/rip-regler-un-support-et-ses-modes.md)
- [Ajouter une machine](../rip-machines/rip-ajouter-une-machine.md)
