---
titre: Composer une planche à la main ou avec des données variables
description: Posez chaque copie au millimètre, utilisez un gabarit de cases ou imprimez une valeur par ligne d'un CSV, par l'API des planches.
sous_rubrique: rip-travaux
position: 67
mots_cles: planche manuelle, gabarit, données variables, csv, api, magnétisme, grille, verrou
---

L'écran de pose manuelle n'existe pas encore dans les clients natifs : on le pilote par l'API du serveur. Cet article s'adresse aux intégrateurs et aux administrateurs.

## Avant de commencer

Vous savez appeler l'API du serveur avec un compte autorisé. Les gabarits se gardent par nom avec `GET`/`POST /api/gabarits` (administrateur).

## Étapes

1. Choisissez le mode de pose : un seul parmi `motif`, `manuel` et `gabarit`, dans le corps de `POST /api/planches` (ou `POST /api/planches/apercu` pour l'aperçu).
2. Pour une pose **manuelle**, envoyez `manuel` :

   ```json
   { "poses": [{ "copie": 0, "x_mm": 0, "y_mm": 0, "angle": 0, "verrou": true }],
     "grille_mm": 10, "magnetisme_mm": 5 }
   ```

   Les copies sont comptées dans l'ordre des éléments, à partir de 0 ; x et y donnent le coin bas gauche sur la planche.
3. Pour un **gabarit**, envoyez `{ "nom", "largeur_mm", "hauteur_mm", "colonnes", "rangees", "espace_mm" }` : une copie par case, de gauche à droite puis de bas en haut, centrée et couchée si elle ne tient que tournée.
4. Pour des **données variables** (sur une seule page), envoyez `donnees_variables` :

   ```json
   { "csv": "Nom;Numéro\n…", "champs": [{ "colonne": "Nom", "x_mm": 10, "y_mm": 5, "taille_pt": 12 }] }
   ```

5. Contrôlez l'aperçu, puis créez la planche.

## Bon à savoir

- Une copie **verrouillée** est prise telle quelle. Les autres sont calées sur la **grille**, puis **aimantées** (à moins de `magnetisme_mm`) au bord de la laize ou au bord d'une copie déjà posée, à l'espacement près.
- Une copie qui sort de la laize ou en chevauche une autre est refusée (`planche_invalide`, avec le numéro). Les copies sans pose vont en rangées au-dessus.
- Données variables : une copie par ligne du CSV (2000 au plus ; séparateur `;` ou `,` pris dans l'en-tête ; guillemets doubles). Chaque champ écrit la valeur de sa colonne en Helvetica noir, à x, y mm du coin bas gauche de la page.
- Les données variables se combinent avec `gabarit` ou `manuel`.
- Les gabarits passent dans la sauvegarde de la configuration.

## Voir aussi

- [Affiner une planche : formes, répétition et contour global](rip-affiner-une-planche.md)
- [Créer une planche pour regrouper des impressions](rip-creer-une-planche.md)
- [Sauvegarder la configuration du serveur](../rip-administration/rip-sauvegarder-la-configuration.md)
