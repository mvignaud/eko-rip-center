---
titre: Comprendre les échanges du pont ERP
description: Le journal des événements, le rythme des appels, le débit limité et les réponses de l'ERP qui commandent les nouveaux essais du RIP.
sous_rubrique: rip-erp
position: 29
mots_cles: journal, journal.jsonl, en-attente, evenements, débit, 429, retry-after, seq_manquant, réessayer
---

Le RIP écrit chaque événement dans un journal avant de l'envoyer, puis dialogue avec l'ERP à intervalles réguliers. Cet article décrit ce mécanisme pour comprendre pourquoi un envoi tarde ou se répète.

## Avant de commencer

Le pont est activé (voir [Activer le pont ERP](rip-activer-le-pont-erp.md)). Les termes d'instance et de `seq` sont expliqués dans [Comprendre les instances du pont ERP](rip-comprendre-les-instances-du-pont-erp.md).

## Étapes

1. **Journal.** Tant que le pont est activé, chaque événement est écrit dans `pont/journal.jsonl` (une ligne, synchronisée sur le disque) **avant** tout envoi. Une coupure de courant pendant l'écriture laisse au plus une ligne coupée : elle est retirée au redémarrage, rien d'autre ne change.
2. **État du pont.** `pont/etat.json` garde le dernier accusé et le dernier état publié de chaque travail et de chaque sortie.
3. **Demande.** Le RIP interroge `GET <base>/en-attente?instance=<uuid>`, signé : toutes les 30 secondes, toutes les 5 secondes quand une demande attend (par exemple une demande de catalogue), et aussitôt qu'un événement s'ajoute.
4. **Envoi.** Il poste `POST <base>/evenements` : une liste de 100 événements au plus, dans l'ordre de `seq`, signée sur le corps exact, de tout ce qui suit le dernier accusé.
5. **Réponse.** La demande peut contenir des `travaux` (envoyés par l'ERP), des `actions` (annuler, suspendre, reprendre, priorité, échéance, opérateur, réimpression) et des `estimations`.

## Bon à savoir

- **Débit** : une requête par seconde au plus (lecture comprise), soit 60 par minute même en rafale, sous la limite de 120 de l'ERP. Les événements arrivés pendant l'attente partent dans le lot suivant.
- Sur `429`, le RIP attend `Retry-After` (en secondes, 1 heure au plus), sinon 60 secondes, avant tout nouvel appel.
- `200` et `409 seq_manquant` donnent `dernier_seq_traite` : le RIP renvoie tout ce qui suit. Toute autre erreur (401, 403, 404, 422, 429, 5xx, réseau) est affichée dans **Système** avec son code ; le RIP réessaie après 60 secondes, puis toutes les 5 minutes. Il ne saute jamais un événement refusé.
- Les événements accusés sont purgés après 30 jours ; ceux qui ne le sont pas restent jusqu'à leur accusé, quelle que soit la durée de la panne.
- Le champ `annulations`, ancien, n'est plus appliqué ; il est signalé au journal du serveur s'il arrive non vide : utilisez `actions`.
- Pont désactivé : rien n'est journalisé. À l'activation, les travaux et sorties existants sont pris tels quels.
- Le dossier `pont/` n'est pas de la configuration : il n'est pas sauvegardé.

## Voir aussi

- [Résoudre un pont ERP en erreur, remplacé ou en anomalie](rip-resoudre-un-pont-erp-en-erreur.md)
- [Recevoir les travaux envoyés par l'ERP](rip-recevoir-les-travaux-envoyes-par-l-erp.md)
- [Piloter la file depuis l'ERP](rip-piloter-la-file-depuis-l-erp.md)
- [Savoir où sont les données du serveur](../rip-administration/rip-savoir-ou-sont-les-donnees-du-serveur.md)
