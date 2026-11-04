---
titre: Recevoir les travaux envoyés par l'ERP
description: Comprenez comment le RIP télécharge le fichier d'un dossier de l'ERP, crée le travail, groupe les travaux liés et refuse un envoi invalide.
sous_rubrique: rip-erp
position: 30
mots_cles: travaux erp, envoi, ref_erp, prereglage_uuid, sha256, groupe, partie, travail.refuse, empreinte_fausse
---

L'ERP peut déposer un fichier dans le RIP : le travail se crée tout seul avec le réglage enregistré désigné. Cet article décrit l'envoi, la création du travail, les travaux liés et les refus.

## Avant de commencer

Le pont est activé. L'ERP doit connaître l'`uuid` du réglage enregistré à employer (voir [Comprendre les identifiants du pont ERP](rip-comprendre-les-identifiants-du-pont-erp.md)).

## Étapes

1. **Envoi** : l'ERP place dans `travaux` un élément avec `id` (uuid de l'envoi), `ref_erp`, `nom`, `prereglage_uuid`, `copies`, `echeance`, `fichier { chemin, nom, taille, sha256 }` et, pour des travaux liés, `groupe` et `partie`.
2. **Téléchargement** : à chaque demande, chaque envoi jamais pris est téléchargé par `GET <base><fichier.chemin>`, signé comme la demande. Cinq envois au plus par cycle, 4 Go au plus par fichier. Le fichier est vérifié par son `sha256` quand l'ERP le donne, puis gardé dans `pont/recus/<id>/` jusqu'à devenir un travail.
3. **Création** : le travail prend l'uuid de l'envoi, le réglage enregistré désigné par `prereglage_uuid` et sa fiche : commande = `ref_erp`, quantité = `copies`, échéance. Un serveur coupé entre la création et le ménage ne refait pas de second travail.
4. **Accusé** : le premier `travail.etat` du travail porte `envoi_id` et la référence.
5. **Travaux liés** : `groupe` (texte de 200 caractères au plus) et `partie` (rang de 1 à 10 000) vont ensemble ; l'un sans l'autre est un `envoi_invalide`. Les sorties d'une partie se posent dans la file juste après celles des parties de rang inférieur ou égal encore en file, sinon juste avant celles d'un rang supérieur : le groupe passe d'un bloc et dans l'ordre, même si les rendus finissent dans le désordre. Une priorité reste possible.

## Bon à savoir

- **Refus** : le RIP répond `travail.refuse` avec `{ "envoi_id", "ref_erp", "code", "detail" }`. Codes : `envoi_invalide` (élément mal formé, `detail` nomme le champ), `telechargement_impossible` (404), `empreinte_fausse`, `reglage_inconnu`, et les refus de création d'un travail (`format_inconnu`, `pdf_illisible`, `polices_manquantes`…).
- **Doublons** : un envoi pris ou refusé n'est jamais repris, même reproposé ; il est oublié 30 jours après que l'ERP a cessé de le proposer. Une panne (réseau, 5xx, disque) n'est pas un refus : l'envoi est repris au cycle suivant.
- Avec `EKO_RIP_PURGER_ERP_FAITS=1`, un travail venu de l'ERP est effacé dès qu'il est fait, sans attendre la durée de conservation : voir [Modifier les réglages avancés du serveur](../rip-administration/rip-modifier-les-reglages-avances-du-serveur.md).
- Pour agir ensuite sur la file, voir [Piloter la file depuis l'ERP](rip-piloter-la-file-depuis-l-erp.md).

## Voir aussi

- [Comprendre les échanges du pont ERP](rip-comprendre-les-echanges-du-pont-erp.md)
- [Piloter la file depuis l'ERP](rip-piloter-la-file-depuis-l-erp.md)
- [Déposer un fichier dans un dossier surveillé](../rip-travaux/rip-deposer-un-fichier-dans-un-dossier-surveille.md)
