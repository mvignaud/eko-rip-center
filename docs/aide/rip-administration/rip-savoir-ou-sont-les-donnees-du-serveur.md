---
titre: Savoir où sont les données du serveur
description: Retrouvez chaque fichier de données du serveur (machines, supports, comptes, file, travaux, sauvegardes) et ce que la sauvegarde en contient.
sous_rubrique: rip-administration
position: 22
mots_cles: données, dossier, var lib eko-rip, fichiers, json, travaux, sauvegardes, emplacement
---

Tout ce que le serveur garde est dans un seul dossier : `/var/lib/eko-rip` pour le paquet (ailleurs, le dossier donné par `EKO_RIP_DATA`). Cet article décrit le contenu de ce dossier pour l'administrateur qui sauvegarde ou dépanne.

## Avant de commencer

Vous avez un terminal sur le serveur et un accès `sudo`. Ne modifiez pas ces fichiers à la main : passez par le client.

## Étapes

1. Listez le dossier : `sudo ls /var/lib/eko-rip`.
2. Repérez le fichier cherché dans le tableau.

   | Chemin | Contenu |
   |---|---|
   | `machines.json` | catalogue des machines |
   | `tarifs.json` | prix d'encre et de support, HT ou TTC |
   | `reglages.json` | réglages enregistrés |
   | `supports.json` | supports par machine et leurs modes d'impression (identifiants stables) ; repris de `tarifs.json` au premier démarrage |
   | `charges.json` | support chargé dans chaque machine, saisi par l'opérateur ou lu sur l'Epson |
   | `decoupe.json` | réglages de découpe du serveur (préfixes, calques non imprimables, tracés ouverts, tri, après coupe) |
   | `tons_directs.json` | table des tons directs (nom exact, alias, couleur Lab) |
   | `tons_inconnus.json` | tons directs rencontrés mais absents de la table (nombre de travaux, dernière fois), listés dans **Système › Tons directs** |
   | `utilisateurs.json` | comptes (mots de passe hachés, droits 0600) |
   | `erp.json` | pont ERP : empreinte du jeton, secret HMAC, URL de l'ERP, activation, instance (droits 0600) |
   | `cloud.json` | Cloud E-KO RIP : adresse de la plate-forme, site et clé reçus à l'appairage (droits 0600) |
   | `pont/` | pont ERP : journal des événements (`journal.jsonl`), état des envois (`etat.json`) et fichiers reçus ; ce n'est pas de la configuration, il n'est pas sauvegardé |
   | `file.json` | file des sorties et historique |
   | `profils/` | profils ICC déposés |
   | `entrees/<réglage>/` | dossiers surveillés, un par réglage |
   | `travaux/<id>/` | un dossier par travail : source, `job.json`, plaques (`prod/`), aperçus |
   | `sorties/<machine>/` | plaques copiées pour une machine en connexion Dossier (Epson), fichiers de coupe d'une découpeuse en Dossier |
   | `releves/<machine>.jsonl` | relevés SNMP (encres, rouleau) pour la consommation réelle |
   | `sauvegardes/avant-<version>-<date>.tar.gz` | configuration copiée avant chaque mise à jour du paquet (machines, supports, réglages, tarifs, comptes, pont ERP, profils ; 10 gardées, droits 0600) |
   | `*.json.prec` | version précédente de chaque fichier d'état, gardée à chaque écriture |
   | `*.json.illisible` | copie d'un fichier d'état trouvé abîmé après une coupure de courant |
3. Pour sauvegarder, suivez [Sauvegarder la configuration du serveur](rip-sauvegarder-la-configuration.md) : le fichier téléchargé contient le catalogue des machines, les tarifs, les réglages enregistrés, la découpe, les tons directs et les profils ICC.

## Bon à savoir

- Les travaux de plus de `EKO_RIP_CONSERVATION_JOURS` jours (30 par défaut) sont effacés avec leurs sorties ; 0 les garde. Voir [Modifier les réglages avancés du serveur](rip-modifier-les-reglages-avances-du-serveur.md).
- Le fichier des comptes n'est pas dans la sauvegarde téléchargée : copiez `utilisateurs.json` à part ou recréez les comptes.
- Les travaux sont volumineux (plaques en pleine définition) et se refont depuis leurs fichiers sources.
- Chaque fichier d'état garde sa version précédente (`.prec`) : voir [Reprendre après une coupure de courant](rip-reprendre-apres-une-coupure-de-courant.md).

## Voir aussi

- [Sauvegarder la configuration du serveur](rip-sauvegarder-la-configuration.md)
- [Remonter un serveur à partir d'une sauvegarde](rip-remonter-un-serveur.md)
- [Reprendre après une coupure de courant](rip-reprendre-apres-une-coupure-de-courant.md)
- [Partager les dossiers surveillés](rip-partager-les-dossiers-surveilles.md)
