Les situations les plus courantes, de la connexion à l'impression, avec leur cause et la marche
à suivre. Le message exact affiché par le client ou l'interface est repris en gras.

## Connexion

### Le client ne trouve pas le serveur

1. Vérifier que le serveur tourne : `systemctl status eko-rip-serveur` sur le serveur, ou ouvrir
   `http://<adresse>:8640` dans un navigateur : la page d'état doit s'afficher.
2. Sur Mac, autoriser l'accès au **réseau local** (Réglages Système › Confidentialité et
   sécurité › Réseau local).
3. La découverte automatique ne traverse ni les routeurs, ni les VPN, ni certains réseaux Wi-Fi
   invités : saisir l'adresse sous **Saisir une adresse** (`192.168.1.20:8640`).
4. Un pare-feu sur le serveur doit laisser passer le port TCP 8640 (et UDP 5353 pour la
   découverte).

### Le navigateur n'affiche qu'une page d'état

C'est voulu : dans l'atelier, l'interface complète est réservée aux clients. Utiliser le client
(menu **Administration…**) ou l'adresse du site sur le [Cloud](cloud.md).

### « Accès libre » en rouge

Aucun compte n'est déclaré : n'importe quel poste du réseau peut tout faire. Créer un compte
administrateur ([Créer les comptes](serveur.md#créer-les-comptes)).

### « Trop d'essais pour cet utilisateur. »

Cinq mots de passe faux : ce nom est bloqué 5 minutes. Attendre, ou redémarrer le service (ce qui
ferme toutes les sessions). Mot de passe oublié : voir [Comptes](administration.md#comptes).

### « Réservé à l'administrateur du RIP. »

Le compte est **opérateur** : machines, supports, tarifs, profils, réglages enregistrés et
sauvegarde sont réservés à l'administrateur.

## Envoi et rendu

### « Fichier trop volumineux pour le serveur. »

Le fichier dépasse la limite (8 Go par défaut). La relever avec `EKO_RIP_MAX_MO` dans
`/etc/default/eko-rip-serveur`, puis redémarrer.

### Un rendu échoue

Le message est dans le détail du travail (inspecteur) et dans le journal du serveur. Causes
courantes :

- PDF protégé par mot de passe ou abîmé : le réenregistrer depuis le logiciel de mise en page ;
- fichier très grand qui dépasse le délai de rendu (30 min par défaut, `EKO_RIP_DELAI_S`) ;
- disque plein : voir la place libre dans **Système** ;
- **Permission denied** sur tous les rendus sous Ubuntu 26.04 : mettre le serveur à jour
  (0.31.0 ou plus récent).

Les polices non incorporées et les images trop peu définies sont signalées dès l'envoi, dans les
alertes du travail.

### Un très grand format est long

Une page dont le rendu dépasserait 3 Go (par exemple une bâche de 1,6 × 3 m à 720 dpi) est rendue
par bandes puis recollée : c'est plus long, mais rien n'est à régler. Prévoir la place disque.

### « Disque presque plein »

Il reste moins de 20 Go. Supprimer de vieux travaux, réduire la durée de conservation
(`EKO_RIP_CONSERVATION_JOURS`) ou agrandir le disque des données. Sous 5 Go, le serveur efface
lui-même les plus vieux travaux inactifs et refuse ceux qui ne tiendraient pas.

## Impression et découpe

### « La sortie vers la machine a échoué. »

La raison est affichée sous la sortie, dans la file : machine éteinte ou injoignable, port USB
absent, droit refusé. Corriger, puis **Refaire** la sortie depuis l'historique.

### « L'imprimante signale une alerte : impression arrêtée. »

L'imprimante a remonté une alerte bloquante (capot ouvert, cartouche vide…). La lever sur la
machine, puis **Refaire** la sortie.

### « Envoi à l'imprimante impossible » (Epson)

Un ton direct du fichier n'a pas d'encre sur la machine : le renommer `White` ou `Metallic`, le
convertir en CMJN ou l'ajouter à la table des tons directs (**Système › Tons directs**), puis
relancer le travail. En connexion réseau directe, le blanc et l'argent passent pour l'instant
par une machine en connexion Dossier.

### La découpe est décalée par rapport à l'impression

Faire une coupe d'essai, mesurer l'écart et le saisir en **Décalage X / Y** de la découpeuse
(**Système › Machines**). Voir aussi le [repérage pour le Kona](../../fr/utilisation.md#repérage-pour-le-kona).

### Une machine ne peut pas être retirée

**Une sortie attend encore cette machine dans la file.** La passer ou l'annuler d'abord.

### La durée estimée ne correspond pas

Mesurer la cadence réelle (m² par heure sur la laize) et la saisir en **Cadence** de la machine.

## Application mobile et Cloud

### L'application ne reçoit pas d'alertes

Vérifier que les notifications sont autorisées dans les réglages du téléphone, puis les réglages
d'alerte de l'application. Détail : [Application mobile](application-mobile.md).

### Le site est « hors ligne » sur le Cloud

Le serveur n'arrive plus à joindre la plate-forme : vérifier qu'il est allumé et qu'il a accès à
Internet (HTTPS sortant). Il se reconnecte seul dans la minute qui suit le retour du réseau.
Détail : [Cloud E-KO RIP](cloud.md).

## Toujours bloqué ?

Utiliser **Signaler un problème…** dans le client : le rapport part avec le journal du serveur
(masqué) et une référence est affichée. Ou écrire à [contact@e-ko.rip](mailto:contact@e-ko.rip)
en indiquant la version du serveur (vue **Système**) et le message exact.
