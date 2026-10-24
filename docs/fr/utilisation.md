# Utilisation

On travaille dans le **client E-KO RIP Center** (voir son installation). L'écran se lit
comme une file de RIP :
- en haut, la **barre d'outils** : **Importer**, puis les actions sur le travail choisi
  (**Imprimer**, **Découper**, **Éditer**, **BAT**, **Relancer**, **Supprimer**), la
  recherche, le thème (sombre ou clair, ◐) et la langue ;
- à gauche, les **vues** (Travaux, File d'attente, Système), les **machines** du
  catalogue avec le nombre de sorties en file (un clic ouvre leur file) et les
  **réglages enregistrés** (un clic ouvre l'import avec ce réglage) ;
- au centre, le **tableau des travaux** ; à droite, l'**inspecteur** du travail choisi.

Si le serveur a des comptes, le client demande d'abord de se connecter (voir *Sécurité*
dans l'installation du serveur).

## Envoyer un travail

1. **Importer** : déposer un fichier dans la zone en pointillés, ou cliquer dessus pour
   le choisir. On peut aussi lâcher un ou plusieurs fichiers n'importe où sur la
   fenêtre : ils partent tous avec les réglages de l'import.
2. Choisir le **Mode** :
   - **Imprimer (et découper)** : rendu en plaques d'encre, plus la découpe s'il y a
     des tracés de découpe ;
   - **Découper seulement** : rien n'est imprimé ; seuls les tracés partent au Kona.
3. Régler la **Résolution** (360, 720 ou 1440 dpi), les **Tracés à découper** et le
   **Repérage Kona**. Les blocs **Couleur** et **Mosaïque** s'ouvrent d'un clic.
4. **Envoyer au RIP**.

Formats acceptés : PDF, AI (enregistré avec « Créer un fichier compatible PDF ») et SVG
(découpe seulement).

## Édition rapide

Bloc **Édition rapide** du panneau d'envoi, ou bouton **Éditer** d'un travail fini (le
rendu est refait depuis le fichier d'origine, sans le renvoyer) :
- **Échelle** (%), **Rotation** (0, 90, 180, 270°), **Miroir** (vitrophanie, caisson vu
  de l'arrière), **Marge blanche** autour de l'image ;
- **Œillets** : repères imprimés (anneau et croix de centre) le long des bords cochés,
  à la **distance au bord** donnée, coins compris, répartis régulièrement avec un **écart
  maximal** ;
- **Contour de découpe rectangulaire** autour de l'image, décalé de quelques mm
  (négatif = vers l'intérieur).

La découpe suit l'édition. Un travail qui a une sortie en file ne s'édite pas : annuler
ou finir la sortie d'abord.

## Mise en file automatique

Bloc **Mise en file automatique** : une fois le rendu fini, chaque page imprimable part
dans la file de l'Epson et chaque page à découper dans celle du Kona. **Ajouter en
suspendu** les met en attente : on importe dix fichiers et on libère ceux qu'on veut
depuis la file. Une découpe qui suit une impression est toujours ajoutée suspendue (le
support doit d'abord passer au Kona). Le réglage s'enregistre avec les autres : un
dossier surveillé peut ainsi imprimer seul.

On peut choisir plusieurs fichiers à la fois ; ils partent chacun avec les mêmes réglages.

## BAT (bon à tirer)

Lien **BAT (PDF)** d'un travail fini : un PDF A4 paysage à envoyer au client, une page
par page du travail : l'image en épreuve écran du profil de support, le tracé de découpe
en magenta (non imprimé), le format en mm, les réglages (mode, résolution, profil, blanc,
édition, œillets), les alertes de pré-vérification et un cadre « Bon pour accord » (date,
nom et signature). Il sort dans la langue de l'écran.

## Visionneuse

Un clic sur une vignette (composite, plaque, découpe) l'ouvre en grand, avec ses
dimensions en mm.

## Suivre les travaux

Chaque travail est une carte : vignette, nom, badges (état, mode ou résolution, nombre
de pages, repérage) et date. États : *En attente*, *En cours*, *Terminé*, *Échec*. Un
clic sur la carte l'ouvre :

- une ligne par page, avec le nombre de tracés de découpe ;
- les boutons **Imprimer (Epson)** et **Découper (Kona)**, qui mettent la page dans la
  file d'attente de la machine ; le badge à côté donne l'état de la dernière sortie ;
- **Fichier .plt** : télécharge la découpe (HP-GL, 0,010 mm par pas, origine au coin
  bas gauche de la page) ;
- les aperçus : composite (avec l'épreuve du profil de support), découpe, et une
  vignette par plaque.

**Alertes** (badge orange) : à l'envoi d'un PDF à imprimer, le serveur signale les
polices non incorporées (une autre sera prise au rendu : vectoriser le texte ou
incorporer la police) et les images de moins de 100 ppi à leur taille imprimée. Le
travail part quand même : c'est à l'opérateur de juger.

La barre de recherche filtre par nom ; les pastilles **Tous**, **En cours**,
**Terminé** et **Échec** filtrent par état. **Relancer** refait un travail en échec ;
**Supprimer** efface un travail fini (refusé tant qu'une sortie l'attend dans la file).

## File d'attente

Vue **File d'attente** : une colonne par machine. Le nombre de sorties à passer
s'affiche dans l'onglet.

- **Mutoh Kona 1650** : la découpe part par le port du plotter (`/dev/kona`).
- **Epson SC-S80600** : son pilote n'existe pas encore. En attendant, les plaques à
  imprimer sont copiées dans `/var/lib/eko-rip/sorties/epson/<travail>-p<page>-<sortie>/`
  sur le serveur. Les tons de découpe (`CutContour`…) n'y sont jamais copiés.

Chaque machine passe une sortie à la fois, dans l'ordre de sa colonne. Sur une sortie
à passer : **En tête**, **Monter**, **Descendre**, **Suspendre** / **Reprendre**,
**Annuler**. Une sortie suspendue ne part pas ; les suivantes passent devant elle.
L'**Historique** garde les sorties finies : **Refaire** relance la même sortie sans
refaire le rendu, **Retirer** l'efface de l'historique.

Une sortie coupée par un arrêt du serveur ne repart pas seule (une découpe ne se
reprend pas au milieu) : elle passe en *Échec*, et **Refaire** la relance.

## Les plaques

Une plaque par encre : Cyan, Magenta, Jaune, Noir, plus une plaque par ton direct
du PDF. Pour l'Epson SC-S80600 :

- **Blanc** : nommer le ton direct `White` dans Illustrator ;
- **Argent métallisé** : nommer le ton direct `Silver`.

Sur l'aperçu d'une plaque, le noir est l'encre et le blanc l'absence d'encre.

Les tons de découpe (`CutContour`…) ne sont jamais imprimés et n'effacent pas l'encre
sous eux, même sans surimpression dans le fichier : au rendu, ils deviennent le colorant
`None` de la norme PDF. Il n'y a donc pas de liseré blanc le long de la découpe.

## Blanc automatique

Bloc **Blanc** (mode Imprimer) : la plaque `White` est générée après le rendu.
- **Sous-couche sous l'encre** : blanc partout où une encre dépasse l'**encre minimale**
  (5 % par défaut) — pour imprimer sur support transparent ou coloré.
- **Aplat sur toute la page** : blanc sur toute la page.
- **Rétraction** (0,2 mm par défaut, jusqu'à 5 mm) : le blanc recule d'autant depuis le
  bord de l'encre (et du bord de la page), pour ne pas dépasser de la couleur.

Le blanc déjà dessiné dans le fichier (ton direct `White`) est gardé : le blanc final
réunit les deux. La plaque générée apparaît dans les aperçus, compte dans le coût et part
à l'Epson avec les autres plaques. Un réglage enregistré garde le choix de blanc.

## Découpe

Les tracés peints dans un ton de découpe sont repérés : `CutContour`, `Thrucut`,
`Kisscut`, `Insidecut`, `Perforation`, `Creasing`, `Score`, `Vcut` (majuscules,
espaces et tirets indifférents). Un ton de découpe à 0 % n'est pas découpé. Avec
**Tous les tracés**, chaque tracé du fichier est découpé (fichiers de découpe seule).
Les tracés sont rognés à la page : la lame ne sort jamais du support.

Ordre de coupe : rainage (`Creasing`, `Score`), perforation et V-cut d'abord, la coupe
en dernier, quand la matière tient encore. Un contour contenu dans un autre (le trou d'un
« O », une fenêtre) est découpé avant lui, pour que la pièce ne bouge pas ; à profondeur
égale, la lame va au contour le plus proche.

## Repérage pour le Kona

En mode **Imprimer**, le choix **Repérage Kona** ajoute autour de l'image :
- **Croix P1–P4** : quatre croix aux coins, à 10 mm de l'image ; sur le Kona,
  Actions > Contour Cut > Manual, pointer P1, P2, P3, P4.
- **Cadre Mutoh (type 1)** : cadre noir de 5 mm, à 10 mm de l'image, intérieur d'au
  moins 250 × 250 mm, blancs de 25 mm sur les côtés, 30 mm à l'avant, 80 mm à
  l'arrière (cotes minimales du manuel Kona respectées). Le Kona le mesure seul.

La page imprimée est agrandie d'autant, et le fichier de découpe suit le même décalage.
Le Kona doit être réglé sur une résolution de **0,010 mm**.

## Réglages et dossiers surveillés

**Enregistrer…** garde tout le panneau sous un nom (lettres, chiffres, espaces, `-`,
`_`, `+`). Le choisir dans **Réglage** remplit le panneau ; **Supprimer** le retire.

Chaque réglage a un **dossier surveillé** : `/var/lib/eko-rip/entrees/<nom du réglage>/`
sur le serveur. Un PDF, AI ou SVG déposé dedans devient un travail avec ce réglage, dès
que sa copie est terminée (taille stable pendant 5 s). Un fichier refusé est rangé dans
`refuses/`, avec un fichier `.refus.txt` qui donne la raison.

## Mosaïque

Bloc **Mosaïque** : cocher **Couper en lés si plus large que la laize**, régler la laize
et le **recouvrement** (zone imprimée sur les deux lés voisins, pour coller ou souder).
Les lés ont tous la même largeur. Chaque lé devient une page du travail, avec sa propre
découpe rognée à son bord. **Numéro et traits d'assemblage** ajoute en pied de chaque lé
une bande de 10 mm : « Lé 2/3 » et un trait vertical là où le lé voisin s'arrête ou
commence, pour aligner les lés à la pose.

## Planche (imbrication)

Sur chaque page imprimable, cocher **Ajouter à la planche** et indiquer le nombre de
copies. Le panneau **Planche** apparaît au-dessus de la liste : régler la **laize**
(largeur utile du rouleau, 1600 mm par défaut) et l'**espacement**, puis **Créer la
planche**. Avant de la créer, le panneau dessine l'**imposition** : le rouleau à la laize,
chaque élément à sa place (survol : nom et dimensions), la longueur de rouleau et la part
de surface utilisée ; il se met à jour à chaque changement. Avec **Tourner de 90° si la planche est plus courte**, quatre orientations
sont essayées (telle quelle, tout tourné, tout couché, tout debout) et la plus courte en
rouleau est gardée ; un élément plus large que la laize n'y passe que tourné. La
résolution et le repérage du panneau Nouveau travail s'appliquent. Les
éléments sont rangés du plus haut au plus bas, de gauche à droite ; la planche est un
nouveau travail, avec la découpe de tous les contours.

Avec **Imbriquer sur la forme des contours de découpe**, chaque élément occupe la place de
son contour de découpe, plus l'espacement, au lieu de son rectangle : ronds en quinconce,
triangles tête-bêche (avec la rotation, les angles 0, 90, 180 et 270° sont essayés). Un
élément sans contour fermé garde son rectangle. L'imposition dessine alors les contours.
Sur une grille de pas égal au 1/800 de la laize (1 mm au moins) : une découpe ne touche
jamais sa voisine, mais l'espace réel peut dépasser l'espacement demandé d'un pas. La part
de surface utilisée compte l'aire des contours de découpe, dans les deux modes.

## Couleur

Bloc **Couleur** (mode Imprimer) :
- **Ajouter un profil ICC…** dépose un profil `.icc` ou `.icm` sur le serveur (profils de
  support, profil d'entrée Epson Wide Gamut…).
- **Entrée CMJN / Entrée RVB** : profil appliqué aux données sans profil (ou à toutes,
  avec **Ignorer les profils du PDF**).
- **Support (sortie)** : profil du support imprimé ; l'aperçu simule son rendu.
- **Intention** : perceptuelle, colorimétrique relative, saturation, absolue.

Les tons directs (`White`, `Silver`…) ne sont jamais convertis.

## Système

Vue **Système** : version du serveur, durée de conservation des travaux (30 jours par
défaut, réglée par `EKO_RIP_CONSERVATION_JOURS`) et espace libre du disque des données,
en rouge sous 10 %.

## Machines

**Système > Machines** : le catalogue des machines de l'atelier. Pour chacune : un
identifiant (minuscules, chiffres, `-`), un nom, le modèle (Epson SC-S80600, Mutoh Kona,
Graphtec FC7000, Roland VersaCAMM VP), la laize, la connexion (série ou USB-série,
réseau TCP, périphérique comme `/dev/usb/lp0`, ou dossier), l'adresse SNMP pour lire le
média chargé, et le pas de découpe (0,01 mm pour le Kona, 0,025 mm par défaut ailleurs).
**Enregistrer** applique tout de suite : les boutons d'impression et de découpe, la file
et la barre latérale suivent. Une machine qui a encore une sortie à passer ne peut pas
être retirée.

- **Graphtec FC7000-75** : régler la machine en HP-GL (Menu > Interface > Command), pas de
  0,025 mm ; le repérage ARMS se règle au panneau de la machine.
- **Roland VersaCAMM VP-300 / VP-540** (762 / 1371 mm) : la découpe part par le réseau ;
  à éprouver à l'atelier. L'impression Roland viendra après une capture réseau.

## Média chargé dans l'Epson

**Système > Média chargé dans l'Epson** : la largeur du rouleau chargé, que l'imprimante
mesure au chargement, est lue par le réseau (SNMP, Printer-MIB normalisée) et devient la
laize des panneaux Planche et Mosaïque (**Utiliser comme laize**, **Relire**). Il faut
renseigner l'adresse IP de l'Epson dans `EKO_RIP_EPSON_SNMP` (voir l'installation du
serveur). À valider sur la machine : si la SC-S80600 ne publie pas sa largeur dans la
Printer-MIB, l'écran le dit, et la lecture passera par une autre voie après capture.

## Encre et coût

Chaque page imprimée indique sa surface, l'encre estimée et son coût :
`0,42 m² · encre 3,1 ml · ≈ 1,92 € (encre 0,44 €, support 1,48 €)`. Sous chaque
vignette de plaque, son taux d'encre (part de la page couverte à 100 %).

Le calcul : pour chaque plaque, *taux × surface × ml/m² à 100 %* donne les ml, puis
*ml ÷ 1000 × prix au litre* le coût ; le support coûte *surface × prix au m²*. Les tons
de découpe ne comptent pas.

Les tarifs se règlent dans **Système > Coûts d'encre et de support** : une ligne par
encre (ml/m² à 100 % et € par litre), une ligne **Toutes les autres** pour les cases
vides, et le prix du support au m². Les valeurs livrées (10 ml/m², cartouche Epson de 700 ml à 123,90 € TTC soit 177 €/l, support HEXIS V3000WG à 1,97 €/m² en rouleau de 1600 mm × 45 m) sont des
valeurs de départ : les remplacer par celles de l'atelier (pesée des cartouches avant
et après un tirage connu, factures). Un changement de tarif s'applique aussi aux
travaux déjà faits.

**Cartouches** : saisir la contenance (ml) et le prix d'une cartouche ; le prix au litre
en est tiré. **Supports (rouleaux)** : un catalogue, chaque support avec son prix au m²,
ou sa largeur, sa longueur et le prix payé du rouleau (le prix au m² est calculé). Le
support se choisit à l'envoi (**Support (prix de revient)**, gardé dans les réglages) ;
avec une largeur connue, la matière comptée est toute la largeur du rouleau sur la
longueur imprimée. La carte du travail affiche son **prix de revient** (encre + matière).

**HT ou TTC** : indiquer si les prix saisis sont **TTC** ou **HT**, le taux de **TVA**
(20 % par défaut) et la base du **prix de revient affiché** (HT par défaut) ; la carte,
l'estimation de chaque page et le CSV (colonne `base`) suivent.

**Exporter la consommation (CSV)** télécharge une ligne par page imprimée (date UTC,
travail, page, surface, ml par encre, coûts), séparateur `;`, décimales avec un point,
pour un tableur ou E-KO ERP.

Limite connue : le support compté est la surface de la page, pas la longueur de rouleau
consommée sur toute la laize.

## Ce qui n'existe pas encore

- Le pilote de l'Epson : les plaques sortent dans un dossier, pas vers l'imprimante.
- La lecture automatique du cadre par le Kona n'a pas encore été essayée sur la machine.
- Les couleurs métallisées `MT-*` du nuancier Epson ne sont pas encore converties.

## Glossaire

- **Plaque** : image 8 bits d'une encre (0 = pleine encre, 255 = pas d'encre).
- **Ton direct** : couleur nommée du PDF (`White`, `Silver`, `CutContour`…), sortie sur
  sa propre plaque au lieu d'être mélangée au CMJN.
- **Laize** : largeur utile du rouleau.
- **Lé** : bande verticale d'une mosaïque, au plus large comme la laize.
- **Sortie** : une page d'un travail envoyée à une machine, dans la file d'attente.
- **Dossier surveillé** : dossier du serveur dont chaque fichier déposé devient un travail.
