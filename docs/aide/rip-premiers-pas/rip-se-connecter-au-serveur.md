---
titre: Se connecter au serveur depuis un poste
description: Choisissez votre serveur dans la liste du réseau local, ou saisissez son adresse, puis ouvrez votre session dans le client.
sous_rubrique: rip-premiers-pas
position: 6
mots_cles: connexion, serveur, adresse, découverte, session, changer de serveur
---

Au premier lancement, le client doit savoir à quel serveur parler. Cet article explique comment le choisir, et comment en changer plus tard.

## Avant de commencer

Le client est installé et le serveur tourne. Si le serveur a des comptes, munissez-vous de votre nom d'utilisateur et de votre mot de passe.

## Étapes

1. Lancez le client. L'écran de connexion cherche seul les serveurs du réseau local.
2. Dans **Serveurs trouvés sur le réseau** (nom, adresse, version), cliquez votre serveur.
3. Si rien n'apparaît, cliquez **Rechercher à nouveau** et patientez environ 3 secondes.
4. Toujours rien : saisissez l'adresse sous **Saisir une adresse**, sous la forme `<adresse-du-serveur>:8640`.
5. Si le serveur a des comptes, saisissez votre nom et votre mot de passe.

## Bon à savoir

- Le client retient le serveur. Pour en changer, utilisez **Changer de serveur…** (sur Mac : <kbd>Cmd</kbd> + <kbd>Maj</kbd> + <kbd>S</kbd>). La liste propose les serveurs trouvés et la dernière adresse saisie.
- Un client ne parle qu'à un serveur à la fois.
- La session dure 12 heures.
- La découverte automatique ne traverse ni les routeurs ni les VPN : saisissez alors l'adresse.
- De l'extérieur de l'atelier, saisissez l'adresse du site sur le Cloud, ou utilisez le bouton **Se connecter au cloud** (clients Mac et Linux).

## Voir aussi

- [Le client ne trouve pas le serveur](../rip-depannage/rip-client-ne-trouve-pas-le-serveur.md)
- [Relier le serveur au Cloud](../rip-cloud-premiers-pas/rip-cloud-relier-un-serveur.md)
- [Créer les comptes du serveur](../rip-administration/rip-creer-les-comptes.md)
- Page détaillée : https://e-ko.rip/documentation/clients/
