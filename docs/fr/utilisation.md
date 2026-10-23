# Utilisation

1. Ouvrir l'écran (client ou navigateur).
2. Choisir un PDF et la résolution (360, 720 ou 1440 dpi), puis **Envoyer au RIP**.
3. Le travail apparaît dans la liste : *En attente*, *Rendu en cours*, puis
   *Terminé* avec l'aperçu de chaque plaque.

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

## Ce qui n'existe pas encore

Les plaques ne partent pas encore vers l'imprimante. Le fichier de découpe ne part pas
encore tout seul vers le plotter : il faut l'envoyer à la main. Les
couleurs métallisées `MT-*` du nuancier Epson ne sont pas encore converties.
