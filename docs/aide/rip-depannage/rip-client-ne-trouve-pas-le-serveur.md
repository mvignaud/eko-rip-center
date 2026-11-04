---
titre: Le client ne trouve pas le serveur
description: Que faire quand la liste des serveurs reste vide ou que la connexion échoue : vérifications et saisie de l'adresse.
sous_rubrique: rip-depannage
position: 1
mots_cles: serveur introuvable, découverte, réseau local, pare-feu, mac, adresse, port 8640
---

Le client liste les serveurs du réseau local dans **Serveurs trouvés sur le réseau**. Si le vôtre n'y est pas, suivez ces vérifications dans l'ordre.

## Avant de commencer

Vous êtes sur le réseau de l'atelier. Si vous avez un accès au serveur, ouvrez un terminal dessus.

## Étapes

1. Vérifiez que le serveur tourne : `systemctl status eko-rip-serveur` sur le serveur.
2. Depuis votre poste, ouvrez `http://<adresse-du-serveur>:8640` dans un navigateur. La page d'état doit s'afficher.

   ![Page d'état du serveur, avec sa version, ses machines et l'adresse à saisir dans le client](../images/serveur-page-etat.png)
3. Sur Mac, autorisez le client à accéder au **réseau local** : Réglages Système › Confidentialité et sécurité › Réseau local.
4. Dans le client, cliquez **Rechercher à nouveau**.
5. Toujours rien : saisissez l'adresse sous **Saisir une adresse**, sous la forme `<adresse-du-serveur>:8640`.
6. Si un pare-feu tourne sur le serveur, laissez passer le port TCP 8640 et le port UDP 5353 (découverte).

## Bon à savoir

- La découverte automatique ne traverse ni les routeurs, ni les VPN, ni certains réseaux Wi-Fi invités.
- Le client Windows ne cherche pas encore les serveurs : il faut toujours saisir l'adresse.
- Le navigateur n'affiche qu'une page d'état : c'est voulu. Pour l'interface complète, utilisez le client.
- Ne jamais ouvrir le port 8640 sur Internet : la connexion n'est pas chiffrée. Pour travailler à distance, utilisez le Cloud.

## Voir aussi

- [Se connecter au serveur depuis un poste](../rip-premiers-pas/rip-se-connecter-au-serveur.md)
- [Connexion refusée](rip-connexion-refusee.md)
- [Relier le serveur au Cloud](../rip-cloud-premiers-pas/rip-cloud-relier-un-serveur.md)
- Page détaillée : https://e-ko.rip/documentation/depannage/
