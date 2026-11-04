---
titre: Suivre une planche en préparation dans la file
description: Lisez la planche en préparation, ce qui s'imbrique ensemble, la longueur et le remplissage, et lancez-la avec Imprimer maintenant.
sous_rubrique: rip-travaux
position: 68
mots_cles: imbrication, planche en préparation, imprimer maintenant, planche abandonnée, déclencheur, remplissage
---

Quand une imprimante imbrique ses impressions, la planche se prépare dans la file avant de partir. Cet article explique comment la lire et la lancer.

## Avant de commencer

L'imbrication est réglée sur l'imprimante (voir [Régler l'imbrication d'une imprimante](../rip-machines/rip-regler-l-imbrication-d-une-imprimante.md)).

## Étapes

1. Ouvrez la vue **File d'attente** (ou **Production**) : la colonne de l'imprimante affiche « Imbrication : *stratégie* » et, au-dessus de ses sorties, chaque **Planche en préparation — n pages**.
2. Lisez la planche : le support, la découpeuse (« aucune ») et le miroir, et **contour NestTrimBox** s'il est demandé ; le dessin de l'imposition ; **Longueur … mm · remplissage … %**.
3. Lisez la liste des pages : nom, page, copies, commande et échéance. Décochez **Imprimer cette page** pour sortir une page de la planche et de la file, jusqu'à ce qu'elle soit recochée.
4. Cliquez **PDF de la planche** pour voir le PDF exact qui sera imprimé.
5. Attendez le déclencheur, ou cliquez **Imprimer maintenant** : la planche part sans attendre.

## Bon à savoir

- *Déclencheur atteint : la planche part.* s'affiche quand c'est le cas.
- Quand la planche part, elle devient un travail « Imbrication *machine* — n page(s) », rendu puis mis en file sur l'imprimante (et sur la découpeuse du groupe s'il en a une). Sa fiche reprend les commandes posées, l'échéance la plus proche et, en notes, chaque page avec ses copies. Les impressions posées passent **Imbriquée**.
- **Ce qui s'imbrique ensemble** : seuls partagent une planche les travaux qui ont le même **support**, le même **mode d'impression**, la même **découpeuse** (« aucune » comprise : un travail à couper au Kona et un travail sans découpe font deux planches) et le même **miroir** ; et ce que la planche n'a qu'une fois : la **résolution**, le **repérage**, la **couleur** (profils, intention), le **blanc**, les couches d'encre, le **Fond sous les repères**, les **Tracés à découper** et la **Tolérance de découpe** ; enfin les **réglages de coupe choisis par couche**. Une couche décochée sur un travail n'est pas coupée sur la planche.
- Seuls les PDF s'imbriquent ; une page plus large que la laize n'entre dans aucune planche.
- **Planche abandonnée** : rendu en échec, travail de planche supprimé, ou impression de la planche annulée, en échec ou retirée avant d'être imprimée. Ses travaux reviennent d'eux-mêmes en file, dans leur état d'avant (une découpe suspendue revient suspendue), et dans la planche en préparation. Rien ne revient d'une planche imprimée.
- Une planche abandonnée ne se refait plus : **Refaire** sa sortie, **Relancer** son travail ou le remettre en file est refusé (*Planche abandonnée : ses travaux sont revenus en file…*), sinon ses travaux s'imprimeraient deux fois. **Imprimer maintenant** sur la planche en préparation en refait une juste. Une planche imprimée, elle, se **Refait** (réimpression voulue).

## Voir aussi

- [Régler l'imbrication d'une imprimante](../rip-machines/rip-regler-l-imbrication-d-une-imprimante.md)
- [Régler les déclencheurs de départ d'une planche](../rip-machines/rip-regler-les-declencheurs-de-depart-d-une-planche.md)
- [Gérer la file d'une machine](rip-gerer-la-file-d-une-machine.md)
