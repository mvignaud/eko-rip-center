---
titre: Résoudre un rendu en échec ou un fichier refusé
description: Trouvez la cause d'un rendu en échec, d'un fichier trop volumineux ou d'un disque presque plein, puis relancez le travail.
sous_rubrique: rip-depannage
position: 3
mots_cles: rendu, échec, fichier trop volumineux, disque plein, permission denied, polices, relancer
---

Un rendu peut échouer à cause du fichier, du délai ou de la place disque. Cet article aide à trouver la cause.

## Avant de commencer

Vous voyez le travail en échec dans le client. Pour modifier un réglage du serveur, il faut un accès `sudo` sur le serveur.

## Étapes

1. Cliquez le travail et lisez le message dans l'inspecteur.
2. Regardez aussi le journal du serveur : `journalctl -u eko-rip-serveur -f`.
3. Si le PDF est protégé par mot de passe ou abîmé, réenregistrez-le depuis le logiciel de mise en page.
4. Si le message est « Fichier trop volumineux pour le serveur. », relevez la limite avec `EKO_RIP_MAX_MO` dans `/etc/default/eko-rip-serveur` (8 Go par défaut).
5. Si le rendu dépasse le délai (30 minutes par défaut), relevez `EKO_RIP_DELAI_S` dans le même fichier.
6. Redémarrez le service : `sudo systemctl restart eko-rip-serveur`.
7. Dans le client, cliquez **Relancer** sur le travail.

## Bon à savoir

- « Disque presque plein » : il reste moins de 20 Go. Supprimez de vieux travaux, réduisez la durée de conservation (`EKO_RIP_CONSERVATION_JOURS`) ou agrandissez le disque. Sous 5 Go, le serveur efface lui-même les plus vieux travaux inactifs.
- « Permission denied » sur tous les rendus sous Ubuntu 26.04 : mettez le serveur à jour.
- Les polices non incorporées et les images trop peu définies sont signalées dès l'envoi, dans les alertes du travail.
- Un très grand format (rendu de plus de 3 Go) est rendu par bandes puis recollé : c'est plus long, mais rien n'est à régler. Prévoyez la place disque.
- Un redémarrage du serveur interrompt le rendu en cours : relancez le travail.

## Voir aussi

- [Envoyer un travail au RIP](../rip-travaux/rip-envoyer-un-travail.md)
- [Mettre à jour le serveur](../rip-administration/rip-mettre-a-jour-le-serveur.md)
- [Choisir le poste serveur](../rip-premiers-pas/rip-choisir-le-poste-serveur.md)
- Page détaillée : https://e-ko.rip/documentation/depannage/
