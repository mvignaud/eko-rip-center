---
titre: Reprendre après une coupure de courant
description: Que devient le travail après une coupure : travaux interrompus à relancer, fichiers d'état repris automatiquement, restauration si un fichier est illisible.
sous_rubrique: rip-administration
position: 23
mots_cles: coupure de courant, interrompu, fichiers d'état, prec, illisible, onduleur, restaurer, relancer
---

Une coupure de courant interrompt le rendu et les envois en cours, mais les fichiers de réglage résistent. Cet article dit quoi vérifier au redémarrage.

## Avant de commencer

Le PC serveur est rallumé et le service a redémarré seul. Vous avez accès à la vue **Système** du client avec un compte administrateur.

## Étapes

1. Dans le client, ouvrez **Administration…** puis la vue **Système**.
2. Cherchez la mention **Fichiers et dossiers** : elle n'apparaît que si un fichier d'état a été trouvé abîmé ou un dossier absent.
3. Si elle dit « <fichier> illisible (coupure ?) : version précédente reprise », rien à faire : le serveur a repris seul la version précédente (`.prec`). Un dossier absent est recréé.
4. Si elle dit « <fichier> illisible, sans version précédente : à restaurer », restaurez la dernière sauvegarde de la configuration : voir [Remonter un serveur à partir d'une sauvegarde](rip-remonter-un-serveur.md). Le fichier abîmé est gardé en copie `.illisible` pour analyse.
5. Revenez à la vue **Production** et relancez avec **Relancer** les travaux passés en échec « interrompu ».
6. Refaites les sorties qui étaient en cours d'envoi : elles sont en échec et se relancent depuis l'historique avec **Refaire**.

## Bon à savoir

- Un travail dont le rendu était en cours passe en échec avec le code `interrompu` ; il ne repart pas seul.
- Une sortie en cours d'envoi passe en échec : une découpe ne reprend pas au milieu, elle est à refaire.
- Chaque fichier d'état est écrit sur le disque avant d'être mis en place, et sa version précédente est gardée.
- Des comptes abîmés n'ouvrent jamais l'accès : la connexion reste exigée.
- Pour limiter les dégâts, branchez le serveur, le switch et si possible la box sur un onduleur qui tient quelques minutes et prévient le serveur pour qu'il s'éteigne proprement (par exemple avec `nut`).

## Voir aussi

- [Choisir le poste serveur](../rip-premiers-pas/rip-choisir-le-poste-serveur.md)
- [Sauvegarder la configuration du serveur](rip-sauvegarder-la-configuration.md)
- [Savoir où sont les données du serveur](rip-savoir-ou-sont-les-donnees-du-serveur.md)
- [Surveiller le service du serveur](rip-surveiller-le-service-du-serveur.md)
