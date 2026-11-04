---
titre: Comprendre le pilote direct de l'Epson
description: Sachez ce que fait le pilote direct (9 encres, tramage, alertes SNMP), ses réglages de tête vide et de plafond d'encre, et ses limites actuelles.
sous_rubrique: rip-machines
position: 45
mots_cles: pilote direct, epson, 9 encres, snmp, tête vide, plafond d'encre, origine, dossier, blanc, argent
---

Avec l'Epson SC-S80600 en connexion **Réseau (TCP)**, le RIP imprime lui-même. Cet article décrit ce que fait ce pilote, pour l'administrateur et les opérateurs.

## Avant de commencer

L'Epson est branchée et déclarée (voir [Brancher l'Epson SureColor SC-S80600](rip-brancher-epson-sc-s80600.md)) : **Adresse IP** de l'imprimante et **Port TCP** 9100.

## Étapes

1. Vérifiez la connexion de la machine : **Réseau (TCP)**, port 9100. Le RIP sépare les plaques C, M, J, N du rendu sur les 9 encres de la machine (couples clair et foncé, limite d'encre), les trame en 4 tailles de point et les envoie bande par bande.
2. Pour les alertes de l'imprimante, renseignez l'adresse **SNMP** de la machine : l'état est lu avant l'envoi, puis toutes les 5 s jusqu'à la fin de l'impression.
3. Réglez l'**Origine X / Y impression (mm)** sur un essai papier : X depuis le bord de référence de la laize (arrondi à 16 points), Y ajouté à la tête vide. 0 par défaut.
4. Imprimez un essai et vérifiez les couleurs, la tête vide et le plafond d'encre.

## Bon à savoir

- Une alerte bloquante (capot ouvert…) arrête l'envoi : la sortie passe en *Échec* avec « L'imprimante signale une alerte : impression arrêtée ». La chauffe compte comme impression en cours. Sans SNMP, la sortie est finie dès que tout est envoyé.
- **Tête vide** : l'image commence 20 mm après le début de la page, pour éviter la bande foncée vue en tête au premier tirage.
- **Plafond d'encre** : chaque canal est limité à 85 %, contre les fines bandes vues dans le jaune au premier tirage.
- Les premiers aplats CMJN imprimés par ce pilote ont donné des couleurs justes.
- Tête vide et plafond sont à confirmer à l'usage ; seuls des tirages de premier essai ont été faits : les autres résolutions et les supports réels restent à éprouver à l'atelier.
- **Blanc et argent** ne passent pas encore par le pilote direct : la mise en file vers une Epson en Réseau est refusée (« Envoi à l'imprimante impossible »). Passez par une machine en **Dossier** en attendant.
- En connexion **Dossier**, rien n'est imprimé : les plaques sont copiées dans `/var/lib/eko-rip/sorties/epson/<travail>-p<page>-<sortie>/` sur le serveur, un fichier par canal de l'imprimante (`K.tif`, `C.tif`, `M.tif`, `Y.tif`, puis `WH.tif` ou `MS.tif`) et `envoi.json` (canaux dans l'ordre, plaque d'origine, couches) ; la sortie finie s'affiche **Déposée dans le dossier (pas imprimée)**.
- Chaque impression affiche sa durée estimée, et l'en-tête de la colonne le total restant.

## Voir aussi

- [Brancher l'Epson SureColor SC-S80600](rip-brancher-epson-sc-s80600.md)
- [La sortie vers la machine a échoué](../rip-depannage/rip-sortie-vers-la-machine-echouee.md)
- [Consulter le journal des impressions](../rip-couleur/rip-consulter-le-journal-des-impressions.md)
