---
titre: Relier son RIP à son ERP : du dossier à la machine
description: Comment un ERP envoie un dossier au RIP et en suit l'état jusqu'à la machine : référence de dossier, statuts remontés, journal signé et quantités réelles.
date: 2026-11-03
categorie: Logiciel
image: /assets/img/linux-travaux.webp
alt: Liste des travaux du client E-KO RIP Center avec la référence de dossier et le statut de chaque travail
---
Relier un RIP à un ERP consiste à faire circuler, sans ressaisie, le dossier de fabrication vers la machine puis l'état et les quantités consommées vers la gestion. Avec E-KO RIP Center, l'ERP envoie un fichier accompagné d'une référence de dossier ; le RIP le rend, l'imprime ou le découpe, et renvoie à l'ERP des événements signés et numérotés. Aucun montant ne transite : le RIP donne des quantités, l'ERP calcule les prix.

## Que signifie « relier son RIP à son ERP » ?

L'ERP porte la commande, le client, le devis et la facturation. Le RIP porte le fichier, la couleur, la file et la machine. Sans lien, quelqu'un recopie le numéro de commande dans le RIP, puis relève à la main l'encre et le support consommés. Le pont ERP supprime ces deux ressaisies.

Le pont d'E-KO RIP Center repose sur un contrat public, `eko-rip-erp/1`. Il est documenté, donc n'importe quel ERP ou outil maison peut s'y brancher. Il est aussi évolutif : un ajout compatible (nouveau champ, nouveau type d'événement) garde la même version, l'ERP ignore ce qu'il ne connaît pas, et un changement incompatible passerait à `/2` après annonce dans le journal des versions.

## Comment les échanges sont-ils protégés ?

Deux principes de sécurité guident le pont.

- **Le RIP sort vers l'ERP en HTTPS** : aucun port n'est ouvert à l'atelier. Les seules routes que l'ERP appelle sur le RIP sont celles du préfixe `/api/erp/`, quand le réseau le permet.
- **Un jeton dédié et une signature**. Un administrateur crée le jeton de l'ERP sur le serveur ; il s'affiche une seule fois, le serveur n'en garde que l'empreinte, et il n'ouvre que les routes du pont. Chaque événement envoyé porte une signature HMAC-SHA256 calculée avec un secret partagé, et l'ERP refuse une signature fausse ou un horodatage trop ancien (plus de cinq minutes), ce qui bloque le rejeu.

Le réglage se fait dans le client, rubrique Système, vue Pont ERP, par un administrateur : adresse de l'ERP (en HTTPS), secret, activation.

## Comment un dossier de fabrication arrive-t-il au RIP ?

L'ERP dépose un envoi qui contient le fichier, la **référence de dossier** (`ref_erp`, par exemple `PROD-2026-00123`), un nom, le réglage enregistré à utiliser, le nombre de copies et une échéance éventuelle. Le RIP télécharge le fichier, vérifie son empreinte SHA-256 quand l'ERP la fournit, puis crée un travail. La fiche du travail reprend la référence comme commande, la quantité et l'échéance.

Les réglages, machines, supports et modes d'impression portent chacun un identifiant unique attribué par le serveur et immuable. L'ERP s'appuie sur cet identifiant, jamais sur un nom, qui peut changer. Le RIP publie son catalogue complet à l'ERP au démarrage du pont et à chaque modification.

Pour un dossier de plusieurs pièces, les travaux liés (un groupe et un rang) passent ensemble et dans l'ordre dans la file, même si les rendus finissent dans le désordre. Un envoi mal formé ou un fichier illisible est refusé avec un code stable et un détail, et jamais perdu en silence.

## Quels statuts le RIP remonte-t-il à l'ERP ?

Le RIP envoie un événement à chaque changement d'état, avec la référence du dossier. Deux niveaux existent.

| Niveau | États |
|---|---|
| Travail (le rendu) | en attente, en cours, terminé (prêt à sortir), échec |
| Sortie (l'envoi à une machine) | en attente, suspendue, en cours, terminée, échec, annulée, imbriquée |

Une sortie est imbriquée quand elle est posée sur une planche d'imbrication. Après une coupure, un travail ou une sortie en cours passe en échec avec le code `interrompu` ; il se relance volontairement, car une découpe ne se reprend pas au milieu.

Une correspondance simple côté ERP : travail terminé = « prêt à imprimer » ; sortie terminée d'une imprimante = « en impression » ; sortie terminée d'une découpeuse = « découpé » ; échec = « bloqué », avec le code. Précision importante : « terminée » signifie aujourd'hui « confiée à la machine », pas « sortie de la machine ».

Le RIP publie aussi l'état relevé des imprimantes (niveaux d'encre, longueur de rouleau restante, support chargé, alertes), seulement quand il change.

## Que contient le journal des événements ?

Avant tout envoi, chaque événement est écrit dans un journal sur disque, avec un numéro de séquence croissant et sans trou. Le RIP envoie par lots de 100 événements au plus, une requête par seconde au maximum, et attend l'accusé de l'ERP. Cela donne trois garanties :

1. **Aucun événement perdu** : après une panne de courant, au plus une ligne coupée est retirée, rien n'est renuméroté. Un événement non accusé reste dans le journal, quelle que soit la durée de la panne.
2. **Aucun doublon** : l'ERP dédoublonne sur l'instance et le numéro de séquence.
3. **Pas de renumérotation silencieuse** : si l'ERP et le RIP ne s'accordent pas, le RIP s'arrête et le signale dans l'écran Système.

Les événements accusés sont purgés après 30 jours. L'écran du pont affiche l'état (actif, en erreur, remplacé, anomalie), le dernier événement accusé, le dernier contact et la dernière erreur.

## Quelles quantités reviennent pour la valorisation ?

À la fin d'une impression puis d'une découpe, le RIP envoie un événement de consommation : surface, longueur de support, gâche, encre par couleur, durée, machine, support et mode. Rien n'est inventé : sans relevé de la machine, l'encre, la longueur et la gâche sont absentes, et l'événement précise si l'encre est mesurée ou estimée. L'ERP applique ses propres prix, puisque aucun montant ne circule.

Le RIP peut aussi estimer un dossier avant fabrication : surface, encre, durée et longueur de rouleau, utiles pour un devis. Là encore, sans prix.

## L'ERP peut-il piloter la file ?

Oui, dans des limites claires. L'ERP peut suspendre, reprendre, annuler un dossier, le passer en tête, modifier l'échéance ou l'opérateur, ou demander une réimpression. Ces actions visent les travaux nés d'un envoi de l'ERP avec cette référence, et seulement leurs sorties encore en file. Un travail créé au RIP n'est jamais touché, et une sortie déjà lancée est laissée telle quelle. Le RIP accuse chaque action en indiquant ce qui a été modifié et ce qui ne l'a pas pu.

Pour la partie machines, la [liste des machines prises en charge](/compatibilite/) précise ce que le RIP pilote. Le [glossaire de la documentation](/documentation/glossaire/) définit les termes du pont. Côté logiciel libre, le contrat public découle de la démarche expliquée dans [pourquoi un RIP open source](/blog/rip-open-source-agpl/).

**Essayez gratuitement** : [téléchargez E-KO RIP Center](/telecharger/) et testez le flux sur votre propre atelier. Un ERP à brancher ou une question sur le contrat ? [Écrivez-nous](/contact/).

## Questions fréquentes

### Le RIP envoie-t-il des prix à l'ERP ?

Non. Aucun montant ne transite, ni coût d'encre ni coût de support. Le RIP envoie des quantités physiques, l'ERP valorise.

### Faut-il ouvrir un port sur la box de l'atelier ?

Non. Le RIP appelle l'ERP en HTTPS sortant. Seules les routes `/api/erp/` sont appelables en entrée, avec le jeton de l'ERP, quand le réseau le permet.

### Que se passe-t-il si l'ERP est indisponible ?

Les événements restent dans le journal du RIP jusqu'à leur accusé, quelle que soit la durée de la panne. Le RIP réessaie après 60 secondes, puis toutes les cinq minutes, sans jamais sauter un événement.

### Mon ERP maison peut-il se brancher ?

Oui, le contrat `eko-rip-erp/1` est public. Il suffit de gérer le jeton, la signature HMAC et les échanges décrits dans la documentation du contrat.
