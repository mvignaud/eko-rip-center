---
titre: Comprendre les identifiants du pont ERP
description: Les machines, supports, modes, couches et réglages portent un UUID immuable que l'ERP doit utiliser à la place des noms.
sous_rubrique: rip-erp
position: 26
mots_cles: uuid, identifiant, immuable, machine, support, mode, réglage, profil icc, identifiant_inconnu
---

Pour que l'ERP s'y retrouve même quand un nom change, chaque élément du catalogue porte un identifiant attribué par le serveur. Cet article explique les règles à respecter.

## Avant de commencer

Il s'adresse à la personne qui développe ou paramètre l'intégration avec l'ERP.

## Étapes

1. Appuyez-vous sur l'`uuid` (UUID v4) de chaque **machine**, **couche de découpe**, **réglage enregistré**, **support** et **mode d'impression**. Il est attribué par le serveur et immuable.
2. Ne vous appuyez jamais sur un nom : le nom affiché et l'identifiant texte d'une machine (`epson`, `kona`…) peuvent changer.
3. N'inventez pas d'identifiant : un `uuid` inconnu envoyé au RIP est refusé avec le code `identifiant_inconnu`.
4. Pour les profils ICC, utilisez leur nom de fichier : ils n'ont pas d'`uuid`.

## Bon à savoir

- Un réglage enregistré **renommé** devient un autre réglage, avec un nouvel `uuid` : son nom est aussi le nom de son dossier surveillé.
- Les identifiants sont publiés dans le catalogue : voir [Consulter les événements envoyés à l'ERP](rip-consulter-les-evenements-envoyes-a-l-erp.md).
- L'identifiant d'une machine sert aussi de nom au dossier `sorties/<machine>/` et au renvoi depuis un autre RIP : ne le changez pas à la légère.

## Voir aussi

- [Comprendre le pont ERP](rip-comprendre-le-pont-erp.md)
- [Consulter les événements envoyés à l'ERP](rip-consulter-les-evenements-envoyes-a-l-erp.md)
- [Ajouter une machine](../rip-machines/rip-ajouter-une-machine.md)
