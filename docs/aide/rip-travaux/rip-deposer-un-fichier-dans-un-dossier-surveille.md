---
titre: Déposer un fichier dans un dossier surveillé
description: Faites partir un fichier au RIP avec un réglage enregistré en le déposant simplement dans le dossier de ce réglage.
sous_rubrique: rip-travaux
position: 6
mots_cles: dossier surveillé, réglage enregistré, dépôt, automatique, règles, entrées
---

Chaque réglage enregistré a son dossier : un fichier déposé dedans devient un travail avec ce réglage, sans ouvrir le client. Cet article s'adresse aux postes qui envoient beaucoup de fichiers identiques.

## Avant de commencer

- Un réglage enregistré existe (bouton **Enregistrer…** du panneau d'envoi, compte administrateur pour les règles).
- Le dossier est accessible depuis votre poste (voir [Partager les dossiers surveillés](../rip-administration/rip-partager-les-dossiers-surveilles.md)).

## Étapes

1. Dans le panneau d'envoi, réglez le travail comme voulu, puis cliquez **Enregistrer…** et donnez un nom au réglage.
2. Ouvrez le dossier du réglage sur le serveur : `/var/lib/eko-rip/entrees/<nom du réglage>/`.
3. Déposez-y un PDF, AI, SVG, TIFF, PNG ou JPEG.
4. Attendez que la copie soit terminée (la taille doit rester stable 5 secondes) : le fichier devient un travail avec ce réglage.
5. Retrouvez le travail dans le tableau du client.

## Bon à savoir

- Un fichier refusé est rangé dans `refuses/`, avec un fichier `.refus.txt` qui donne la raison.
- Le dossier automatique `/var/lib/eko-rip/entrees-auto/` choisit le réglage lui-même d'après les **Règles du dossier automatique** (administrateur, **Système**) : motif du nom, petit côté, grand côté. La première règle qui correspond s'applique.
- Le nom du fichier peut porter le réglage, la machine, les copies, etc., avec un motif à balises comme `{nom}__{machine}__{reglage}__{copies}`.
- La case **Une page par travail** crée un travail par page d'un PDF. La case **Refuser si une police manque** rejette les fichiers dont une police n'est pas incorporée.
- Les dossiers surveillés ne demandent pas de compte du RIP : leur accès se règle par le partage du dossier.

## Voir aussi

- [Partager les dossiers surveillés](../rip-administration/rip-partager-les-dossiers-surveilles.md)
- [Envoyer un travail au RIP](rip-envoyer-un-travail.md)
- [Sauvegarder la configuration du serveur](../rip-administration/rip-sauvegarder-la-configuration.md)
- Page détaillée : https://e-ko.rip/documentation/utilisation/
