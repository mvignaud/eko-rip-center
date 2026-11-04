---
titre: Vérifier la signature des événements du pont ERP
description: Recalculez la signature HMAC-SHA256 d'un événement du RIP, avec le vecteur de contrôle, pour qu'un ERP accepte les événements et refuse les rejeux.
sous_rubrique: rip-erp
position: 25
mots_cles: signature, hmac, sha256, x-eko-rip-signature, enveloppe, événement, rejeu, vecteur de contrôle
---

Chaque événement que le RIP envoie à l'ERP porte une signature. Cet article décrit comment l'ERP la vérifie et ce que contient l'enveloppe d'un événement. Il s'adresse à la personne qui développe ou contrôle l'intégration.

## Avant de commencer

Vous avez le secret partagé (voir [Créer le jeton et le secret du pont ERP](rip-creer-le-jeton-et-le-secret-du-pont-erp.md)), tel qu'affiché : 64 caractères, pris comme octets ASCII.

## Étapes

1. Lisez l'en-tête de l'événement : `X-EKO-RIP-Signature: t=<secondes Unix>,v1=<hex>`.
2. Prenez le **corps exact** reçu, en octets : ne re-sérialisez pas le JSON.
3. Formez le message : `"<t>." + corps`.
4. Calculez `HMAC-SHA256(clé, message)`, où la clé est le secret tel qu'affiché, et écrivez le résultat en hexadécimal minuscule.
5. Comparez avec `v1` en **temps constant**. Refusez l'événement si la signature est fausse.
6. Refusez aussi un `t` à plus de 5 minutes de l'horloge de l'ERP (rejeu).
7. Traitez un `seq` déjà vu comme un doublon : répondez 200 sans rien refaire.
8. Contrôlez votre calcul avec le vecteur de contrôle (RFC 4231, cas 2) : clé `Jefe`, message `what do ya want for nothing?` donne `5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843`.

## Bon à savoir

- Enveloppe d'un événement :

  ```json
  {
    "contrat": "eko-rip-erp/1",
    "instance": "0c9d6a2e-5b1f-4f0e-9a7d-3e2b8c4f6a10",
    "seq": 42,
    "id": "6f1c0e8e-1b8e-4b9a-9d5e-2f6f3c1a7b10",
    "type": "sortie.etat",
    "le": 1793000000,
    "donnees": { }
  }
  ```
- `instance` identifie cette installation du pont ; `seq` est le numéro croissant sans trou ; `id` est l'identifiant unique de l'événement ; `le` est l'heure en secondes Unix ; `type` vaut `catalogue.modifie`, `travail.etat`, `sortie.etat`, `travail.consommation`, `machine.etat`, `travail.refuse`, `travail.action` ou `travail.estimation`.
- Les requêtes de lecture du RIP (`en-attente`, téléchargement d'un fichier) sont signées de la même façon, sur `"<t>.GET <chemin>?instance=<uuid>"`.
- L'ERP ignore les champs et les types qu'il ne connaît pas.
- Une signature refusée côté ERP apparaît côté RIP comme une erreur du pont : voir [Résoudre un pont ERP en erreur, remplacé ou en anomalie](rip-resoudre-un-pont-erp-en-erreur.md).

## Voir aussi

- [Créer le jeton et le secret du pont ERP](rip-creer-le-jeton-et-le-secret-du-pont-erp.md)
- [Comprendre les instances du pont ERP](rip-comprendre-les-instances-du-pont-erp.md)
- [Consulter les événements envoyés à l'ERP](rip-consulter-les-evenements-envoyes-a-l-erp.md)
