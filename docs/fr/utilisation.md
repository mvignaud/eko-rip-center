# Utilisation

L'écran a trois vues, en haut : **Travaux**, **File d'attente** et **Système**. Le
bouton à droite change de langue (français ou anglais). Si le serveur a des comptes,
l'écran demande d'abord de se connecter (voir *Sécurité* dans l'installation du serveur).

## Envoyer un travail

1. Vue **Travaux**, panneau **Nouveau travail** : déposer un fichier dans la zone en
   pointillés, ou cliquer dessus pour le choisir. On peut aussi lâcher un ou plusieurs
   fichiers n'importe où sur la page : ils partent tous avec les réglages du panneau.
2. Choisir le **Mode** :
   - **Imprimer (et découper)** : rendu en plaques d'encre, plus la découpe s'il y a
     des tracés de découpe ;
   - **Découper seulement** : rien n'est imprimé ; seuls les tracés partent au Kona.
3. Régler la **Résolution** (360, 720 ou 1440 dpi), les **Tracés à découper** et le
   **Repérage Kona**. Les blocs **Couleur** et **Mosaïque** s'ouvrent d'un clic.
4. **Envoyer au RIP**.

Formats acceptés : PDF, AI (enregistré avec « Créer un fichier compatible PDF ») et SVG
(découpe seulement).

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

Sur l'aperçu d'une plaque, le noir est l'encre et le blanc l'absence d'encre. Une
plaque de découpe est marquée « découpe, non imprimée ».

## Découpe

Les tracés peints dans un ton de découpe sont repérés : `CutContour`, `Thrucut`,
`Kisscut`, `Insidecut`, `Perforation`, `Creasing`, `Score`, `Vcut` (majuscules,
espaces et tirets indifférents). Un ton de découpe à 0 % n'est pas découpé. Avec
**Tous les tracés**, chaque tracé du fichier est découpé (fichiers de découpe seule).
Les tracés sont rognés à la page : la lame ne sort jamais du support.

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
découpe rognée à son bord.

## Planche (imbrication)

Sur chaque page imprimable, cocher **Ajouter à la planche** et indiquer le nombre de
copies. Le panneau **Planche** apparaît au-dessus de la liste : régler la **laize**
(largeur utile du rouleau, 1600 mm par défaut) et l'**espacement**, puis **Créer la
planche**. La résolution et le repérage du panneau Nouveau travail s'appliquent. Les
éléments sont rangés du plus haut au plus bas, de gauche à droite ; la planche est un
nouveau travail, avec la découpe de tous les contours.

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

## Encre et coût

Chaque page imprimée indique sa surface, l'encre estimée et son coût :
`0,42 m² · encre 3,1 ml · ≈ 1,92 € (encre 0,44 €, support 1,48 €)`. Sous chaque
vignette de plaque, son taux d'encre (part de la page couverte à 100 %).

Le calcul : pour chaque plaque, *taux × surface × ml/m² à 100 %* donne les ml, puis
*ml ÷ 1000 × prix au litre* le coût ; le support coûte *surface × prix au m²*. Les tons
de découpe ne comptent pas.

Les tarifs se règlent dans **Système > Coûts d'encre et de support** : une ligne par
encre (ml/m² à 100 % et € par litre), une ligne **Toutes les autres** pour les cases
vides, et le prix du support au m². Les valeurs livrées (10 ml/m², 140 €/l) sont des
valeurs de départ : les remplacer par celles de l'atelier (pesée des cartouches avant
et après un tirage connu, factures). Un changement de tarif s'applique aussi aux
travaux déjà faits.

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
