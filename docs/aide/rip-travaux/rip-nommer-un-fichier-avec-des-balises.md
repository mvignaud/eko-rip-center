---
titre: Nommer un fichier avec des balises
description: Faites porter au nom du fichier le réglage, la machine, les copies, l'échelle et la laize grâce à un motif à balises, pour un dépôt sans clic.
sous_rubrique: rip-travaux
position: 64
mots_cles: motif à balises, nom de fichier, réglage, machine, copies, échelle, laize, séparateur
---

Un motif à balises lit le nom du fichier déposé pour en tirer le réglage et le reste. Cet article s'adresse à l'administrateur et aux postes qui préparent les fichiers.

## Avant de commencer

Il vous faut un compte administrateur pour définir le motif. Le motif se règle dans le bloc **Règles du dossier automatique** (voir [Régler le dossier automatique](rip-regler-le-dossier-automatique.md)).

## Étapes

1. Dans **Système**, bloc **Règles du dossier automatique**, saisissez le **Motif à balises du nom**, par exemple `{nom}__{machine}__{reglage}__{copies}`.
2. Cliquez **Enregistrer**.
3. Nommez vos fichiers selon le motif : `Affiche__epson__bache__30.pdf` donne le réglage Bâche, l'Epson et 30 copies.
4. Déposez-les dans `entrees-auto/`.

   | Balise | Effet |
   |---|---|
   | `{nom}` | Texte libre, sans effet. |
   | `{reglage}` | Réglage enregistré, sans tenir compte des majuscules, accents, espaces, `-` ni `_` (`bache` = « Bâche »). |
   | `{machine}` | Machine du catalogue, par identifiant ou par nom : imprimante de la mise en file automatique, ou découpeuse du travail. |
   | `{support}` | Support de l'Epson, par nom. |
   | `{copies}` | Quantité de la fiche travail (1 à 100 000). |
   | `{repetition}` | Nombre d'impressions de chaque page mises en file d'office (1 à 100). |
   | `{laize}` | Laize de la mosaïque du réglage, en mm (`1600` ou `1600mm`). |
   | `{echelle}` | Échelle en % (`200`, `12,5%`). |
   | `{largeur}` / `{hauteur}` | Taille voulue en mm, pour un PDF : l'échelle en découle (la plus petite des deux si les deux sont données). |

## Bon à savoir

- Chaque balise prend le texte jusqu'au séparateur qui la suit ; la dernière va jusqu'à l'extension.
- Le séparateur double `__` est conseillé : un `_` seul se trouve souvent dans les noms.
- Deux balises doivent être séparées, et chacune ne sert qu'une fois.
- Un nom qui ne suit pas le motif, ou une valeur qui ne se lit pas, n'est pas refusé : le travail passe par les règles et porte une alerte (« Nom hors du motif à balises », « Balise du nom non reconnue »), aussi écrite dans le journal du serveur.
- Sans `{reglage}` reconnu, ce sont les règles qui choisissent le réglage ; si aucune ne prend le fichier, la raison du refus donne ces alertes.

## Voir aussi

- [Régler le dossier automatique](rip-regler-le-dossier-automatique.md)
- [Renseigner la fiche travail](rip-renseigner-la-fiche-travail.md)
- [Déposer un fichier dans un dossier surveillé](rip-deposer-un-fichier-dans-un-dossier-surveille.md)
