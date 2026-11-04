---
titre: Résoudre une connexion refusée ou un droit manquant
description: Comprenez les messages « Accès libre », « Trop d'essais » et « Réservé à l'administrateur du RIP » et corrigez-les.
sous_rubrique: rip-depannage
position: 2
mots_cles: connexion, mot de passe, trop d'essais, accès libre, administrateur, opérateur, droits
---

Trois messages reviennent souvent autour des comptes. Cet article donne la cause de chacun et la marche à suivre.

## Avant de commencer

Selon le message, vous avez besoin d'un terminal sur le serveur ou d'un compte administrateur.

## Étapes

1. Lisez le message exact affiché.
2. Pour **« Trop d'essais pour cet utilisateur. »** : cinq mots de passe faux ont bloqué ce nom 5 minutes. Attendez, puis réessayez.
3. En cas de mot de passe oublié, un administrateur redéclare le compte avec `utilisateur ajouter` : le nouveau mot de passe remplace l'ancien.
4. Pour **« Réservé à l'administrateur du RIP. »** : votre compte est **opérateur**. Demandez à un administrateur de faire le réglage, ou de vous donner le rôle administrateur.
5. Pour **« Accès libre »** en rouge : aucun compte n'est déclaré. Créez un compte administrateur.

## Bon à savoir

- Machines, supports, tarifs, profils, réglages enregistrés et sauvegarde sont réservés à l'administrateur.
- Redémarrer le service débloque un nom aussitôt, mais ferme toutes les sessions.
- La session dure 12 heures : passé ce délai, le client demande de se reconnecter.

## Voir aussi

- [Créer les comptes du serveur](../rip-administration/rip-creer-les-comptes.md)
- [Gérer les comptes du serveur](../rip-administration/rip-gerer-les-comptes.md)
- [Le client ne trouve pas le serveur](rip-client-ne-trouve-pas-le-serveur.md)
- Page détaillée : https://e-ko.rip/documentation/depannage/
