# Utilisation

1. Ouvrir l'écran (client ou navigateur).
2. Choisir un PDF et la résolution (360, 720 ou 1440 dpi), puis **Envoyer au RIP**.
3. Le travail apparaît dans la liste : *En attente*, *Rendu en cours*, puis
   *Terminé* avec l'aperçu de chaque plaque.

Un PDF de plusieurs pages donne une ligne par page, avec ses aperçus, sa découpe
et son bouton d'envoi au plotter.

## Les plaques

Une plaque par encre : Cyan, Magenta, Jaune, Noir, plus une plaque par ton direct
du PDF. Pour l'Epson SC-S80600 :

- **Blanc** : nommer le ton direct `White` dans Illustrator ;
- **Argent métallisé** : nommer le ton direct `Silver`.

Sur l'aperçu d'une plaque, le noir est l'encre et le blanc l'absence d'encre.

## Découpe

Les tracés peints dans un ton de découpe sont repérés : `CutContour`, `Thrucut`,
`Kisscut`, `Insidecut`, `Perforation`, `Creasing`, `Score`, `Vcut` (majuscules,
espaces et tirets indifférents). La fiche du travail indique leur nombre et propose
**Télécharger le fichier .plt** (HP-GL, 1/40 mm, origine au coin bas gauche de la
page). Un ton de découpe à 0 % n'est pas découpé.

## Mosaïque

Bloc **Mosaïque** : cocher **Couper en lés si plus large que la laize**, régler la laize
et le **recouvrement** (zone imprimée sur les deux lés voisins, pour coller ou souder).
Chaque lé devient une page du travail, avec sa propre découpe rognée à son bord.

## Planche (imbrication)

Sur chaque page imprimable, cocher **Planche** et indiquer le nombre de copies. Le
panneau **Planche** apparaît : régler la **laize** (largeur utile du rouleau, 1600 mm par
défaut) et l'**espacement**, puis **Créer la planche**. La résolution et le repérage du
formulaire s'appliquent. Les éléments sont rangés du plus haut au plus bas, de gauche à
droite ; la planche est un nouveau travail, avec la découpe de tous les contours.

## Couleur

Bloc **Couleur** (mode Imprimer) :
- **Ajouter un profil ICC…** dépose un profil `.icc` ou `.icm` sur le serveur (profils de
  support, profil d'entrée Epson Wide Gamut…).
- **Entrée CMJN / Entrée RVB** : profil appliqué aux données sans profil (ou à toutes,
  avec **Ignorer les profils du PDF**).
- **Support (sortie)** : profil du support imprimé ; l'aperçu simule son rendu.
- **Intention** : perceptuelle, colorimétrique relative, saturation, absolue.

Les tons directs (`White`, `Metallic`…) ne sont jamais convertis.

## Repérage pour le Kona

En mode **Imprimer**, le choix **Repérage Kona** ajoute autour de l'image :
- **Croix P1–P4** : quatre croix aux coins, à 10 mm de l'image ; sur le Kona,
  Actions > Contour Cut > Manual, pointer P1, P2, P3, P4.
- **Cadre Mutoh (type 1)** : cadre noir de 5 mm, à 10 mm de l'image, intérieur d'au
  moins 250 × 250 mm, blancs de 25 mm sur les côtés, 30 mm à l'avant, 80 mm à
  l'arrière (cotes minimales du manuel Kona respectées). Le Kona le mesure seul.

La page imprimée est agrandie d'autant, et le fichier de découpe suit le même décalage.
Le Kona doit être réglé sur une résolution de **0,010 mm**.

## Ce qui n'existe pas encore

Les plaques ne partent pas encore vers l'imprimante. Le fichier de découpe ne part pas
encore tout seul vers le plotter : il faut l'envoyer à la main. Les
couleurs métallisées `MT-*` du nuancier Epson ne sont pas encore converties.
