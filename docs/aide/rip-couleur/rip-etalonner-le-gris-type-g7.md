---
titre: Étalonner le gris (méthode de type G7)
description: Imprimez la mire de gris, envoyez les mesures L*a*b* et obtenez les courbes de correction C, M, J et N, itération après itération.
sous_rubrique: rip-couleur
position: 46
mots_cles: g7, gris, étalonnage, mire, courbes de correction, itérations, engraissement, cgats, api
---

L'étalonnage du gris corrige les courbes d'encre pour obtenir un gris neutre. Il se pilote par l'API et est réservé à l'administrateur. Il ne s'agit pas d'une certification G7 : méthode et valeurs sont celles du RIP.

## Avant de commencer

Vous avez un compte administrateur, un spectrophotomètre et le support de l'étalonnage.

## Étapes

1. Récupérez la mire : `GET /api/g7/mire.cgats?n=21` donne une rampe CMY à parts égales et une rampe noire, de 0 à 100 %.
2. Imprimez la mire, puis mesurez-la (L*a*b* par plage).
3. Envoyez les mesures à `POST /api/g7/iterations` :

   ```json
   {"nom": "Epson / bâche", "mesures": [{"rampe": "cmy", "pct": 50, "l": 58.1, "a": 1.2, "b": -0.4}]}
   ```

   Chaque rampe demande son papier (0 %), son aplat (100 %) et au moins trois plages entre les deux. Vous pouvez ajouter `engraissement` (22 % par défaut à 50 %) et `aplats` (le a*b* des aplats C, M et Y).
4. Lisez la réponse : les écarts à la cible (ΔL* moyen et écart de gris moyen, avec `conforme` sous 1,5 et 1,5, seuils indicatifs), les courbes de correction C, M, Y et K de 5 en 5 %, toutes les itérations de cet étalonnage et le numéro de la meilleure.
5. Imprimez la mire suivante avec les courbes de la meilleure itération, et indiquez-la par `"avec": <son numéro>` à l'envoi des mesures. La nouvelle correction se compose avec ces courbes.

## Bon à savoir

- Une itération moins bonne n'efface pas la meilleure ; les 50 dernières sont gardées.
- Changer `engraissement` ou `aplats` recalcule toutes les itérations sur la nouvelle cible.
- La cible du gris garde la teinte du papier, qui s'efface à mesure que le gris fonce.
- Les courbes s'appliqueront au rendu avec la linéarisation par mode.
- L'étalonnage est compris dans la sauvegarde de la configuration.

## Voir aussi

- [Proposer une limite d'encre à partir de mesures](rip-proposer-une-limite-d-encre.md)
- [Choisir les profils de couleur d'un travail](rip-choisir-les-profils-de-couleur.md)
- [Sauvegarder la configuration du serveur](../rip-administration/rip-sauvegarder-la-configuration.md)
