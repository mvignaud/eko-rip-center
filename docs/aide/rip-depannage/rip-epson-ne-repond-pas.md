---
titre: Résoudre une Epson qui ne répond pas ou sans largeur de média
description: Vérifiez l'adresse SNMP, la communauté et le réseau quand l'Epson ne donne ni son état ni la largeur du média chargé.
sous_rubrique: rip-depannage
position: 25
mots_cles: epson, snmp, communauté, largeur du média, injoignable, 9100, alerte, pilote direct
---

Le serveur lit l'état de l'Epson par SNMP : média chargé, niveaux d'encre, alertes. Cet article aide quand ces informations manquent.

## Avant de commencer

Vous êtes administrateur du serveur, avec un terminal et un accès `sudo`.

## Étapes

1. Vérifiez que l'Epson est allumée et sur le même réseau que le serveur.
2. Vérifiez l'adresse SNMP de la machine : `EKO_RIP_EPSON_SNMP` dans `/etc/default/eko-rip-serveur`.
3. Vérifiez la communauté : `EKO_RIP_EPSON_COMMUNAUTE`, `public` par défaut.
4. Depuis le serveur, testez le port d'impression : `nc -vz <adresse-de-l-imprimante> 9100`.
5. Redémarrez le service : `sudo systemctl restart eko-rip-serveur`.
6. Regardez si la vue **Système**, section **Média chargé dans l'Epson**, affiche de nouveau le média.

## Bon à savoir

- Sans SNMP, le pilote direct ne peut pas arrêter une impression sur une alerte de l'imprimante. Le serveur parle SNMP lui-même : rien à installer.
- Si l'alerte « L'imprimante signale une alerte : impression arrêtée. » apparaît, levez-la sur l'Epson puis cliquez **Refaire** : voir [La sortie vers la machine a échoué](rip-sortie-vers-la-machine-echouee.md).
- Un support chargé non lu peut aussi être choisi à la main par l'opérateur.

## Voir aussi

- [Brancher l'Epson SureColor SC-S80600](../rip-machines/rip-brancher-epson-sc-s80600.md)
- [La sortie vers la machine a échoué](rip-sortie-vers-la-machine-echouee.md)
- [Modifier les réglages avancés du serveur](../rip-administration/rip-modifier-les-reglages-avances-du-serveur.md)
