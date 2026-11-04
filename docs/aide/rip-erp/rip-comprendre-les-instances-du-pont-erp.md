---
titre: Comprendre les instances du pont ERP
description: Comprenez l'instance et le numéro de séquence qui garantissent qu'aucun événement n'est perdu, doublonné ou renuméroté en silence.
sous_rubrique: rip-erp
position: 24
mots_cles: instance, seq, séquence, remplacé, anomalie, dernier_seq_traite, nouvelle instance, remise à zéro
---

Chaque installation du pont porte une **instance** et numérote ses événements sans trou. Cet article explique comment l'ERP s'en sert et ce qui se passe quand deux RIP se disputent la place.

## Avant de commencer

Il s'adresse à l'administrateur du RIP et à l'intégrateur de l'ERP. La mise en place est décrite dans [Activer le pont ERP](rip-activer-le-pont-erp.md).

## Étapes

1. Repérez l'instance : un UUID tiré au premier démarrage du serveur, gardé dans `erp.json` et repris par la sauvegarde de la configuration. Elle s'affiche dans **Système › Pont ERP**.
2. Retenez que `seq` est un numéro croissant **sans trou** pour une instance : l'ERP dédoublonne et détecte les trous sur le couple `(instance, seq)`.
3. Suivez les cas de réponse de l'ERP :
   - instance **jamais accusée**, `dernier_seq_traite = 0` : tout est rejoué depuis `seq = 1` (installation neuve) ;
   - instance **déjà accusée**, réponse `0` : une autre installation a pris la place. Le RIP n'envoie plus rien (ni lecture ni envoi) et affiche « remplacé » ;
   - réponse **plus grande** que le dernier `seq` du RIP pour la même instance, ou rejeu demandé d'événements purgés : « anomalie », arrêt, jamais de renumérotation silencieuse ;
   - instance inconnue de l'ERP qui commence à `seq = 1` : elle ferme la précédente ; avec `seq` supérieur à 1, l'ERP répond `409` `seq_manquant` et `dernier_seq_traite = 0` : le RIP rejoue depuis 1.
4. Pour relancer un RIP arrêté (remplacé ou anomalie), tirez une nouvelle instance dans **Système › Pont ERP**, après vérification avec l'ERP. Le journal repart de 1.

## Bon à savoir

- Seul l'administrateur relance un RIP « remplacé » ou en « anomalie ».
- **Remise à zéro de la séquence depuis l'écran de l'ERP** : l'ERP répond alors 0 pour une instance déjà accusée, et le RIP ne la distingue pas d'un remplacement. Il passe « remplacé » et s'arrête ; tirez une nouvelle instance pour qu'il reprenne (journal rejoué depuis 1).
- L'ERP traite un `seq` déjà reçu comme un doublon : il répond 200 sans rien refaire.
- Une instance neuve rejoue tout l'historique du journal : prévenez l'ERP avant de tirer une nouvelle instance.

## Voir aussi

- [Résoudre un pont ERP en erreur, remplacé ou en anomalie](rip-resoudre-un-pont-erp-en-erreur.md)
- [Comprendre les échanges du pont ERP](rip-comprendre-les-echanges-du-pont-erp.md)
- [Vérifier la signature des événements du pont ERP](rip-verifier-la-signature-des-evenements-du-pont-erp.md)
