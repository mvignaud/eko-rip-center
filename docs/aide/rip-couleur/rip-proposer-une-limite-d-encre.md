---
titre: Proposer une limite d'encre à partir de mesures
description: Imprimez des aplats d'encre croissante, mesurez leur L* et obtenez par l'API la limite à saisir dans le mode, ainsi qu'une mire CGATS.
sous_rubrique: rip-couleur
position: 45
mots_cles: limite d'encre, linéarisation, mire cgats, l*, api, aplats, bronzage
---

Pour fixer la limite d'encre d'un mode, on imprime des aplats d'encre croissante et on mesure leur luminosité. L'API propose la limite. Cet article s'adresse aux administrateurs.

## Avant de commencer

Vous avez un spectrophotomètre pour mesurer L*, et un compte administrateur pour appeler l'API.

## Étapes

1. Récupérez une mire de linéarisation : `GET /api/linearisation/mire.cgats?n=21` (option `encres=C,Lc,M` pour choisir les encres). La mire CGATS donne une plage par encre et par palier, resserrée dans les hautes lumières : 8 plages sous 20 % sur 21, contre 4 pour une rampe régulière.
2. Imprimez des aplats d'encre totale croissante (par exemple de 100 à 400 % par pas de 20 %) et mesurez leur L*.
3. Envoyez la série à `POST /api/linearisation/limite-proposee` :

   ```json
   {"mesures": [[100, 58.0], [120, 47.2]]}
   ```

4. Lisez la réponse : la limite proposée est le dernier palier qui fonce encore d'au moins 1 L* par 10 % d'encre en plus, arrondi aux 5 % inférieurs ; un palier qui s'éclaircit (bronzage, bavure) l'arrête.
5. Relisez les gains renvoyés, vérifiez l'aplat au séchage, puis saisissez la limite dans le mode (voir [Régler les encres claires et l'encre totale d'un mode](rip-regler-les-encres-claires-et-l-encre-totale.md)).

## Bon à savoir

- Rien n'est enregistré par l'appel : c'est vous qui saisissez la limite.
- Pour étalonner le gris, voir [Étalonner le gris (type G7)](rip-etalonner-le-gris-type-g7.md).

## Voir aussi

- [Régler les encres claires et l'encre totale d'un mode](rip-regler-les-encres-claires-et-l-encre-totale.md)
- [Étalonner le gris (type G7)](rip-etalonner-le-gris-type-g7.md)
- [Régler un support et ses modes d'impression](rip-regler-un-support-et-ses-modes.md)
