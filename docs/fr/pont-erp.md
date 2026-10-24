# Pont ERP ↔ RIP — contrat `eko-rip-erp/1` (lot R1)

Ce document fixe ce qu'un ERP peut attendre d'E-KO RIP Center. Le lot R1 pose les
fondations : identifiants, accès machine à machine, signature des événements, modèle
d'états. Les lots suivants (R2 canal et catalogue, R3 réception des travaux, R4 estimation,
R5 consommation réelle) s'ajoutent sans casser ce qui est écrit ici.

Principes :

- **Aucun montant ne transite** : le RIP envoie des quantités physiques (surfaces,
  longueurs, encre, temps), l'ERP valorise.
- **Le RIP sort vers l'ERP** (HTTPS) : aucun port n'est ouvert à l'atelier. Les seules
  routes que l'ERP appelle sur le RIP sont celles du préfixe `/api/erp/`, quand le réseau
  le permet (même site, VPN).

## 1. Version du contrat

- Nom : `eko-rip-erp/1`, renvoyé par `GET /api/erp/contrat` et porté par chaque événement.
- Un ajout compatible (champ, type d'événement, état nouveau documenté) garde `/1`.
  L'ERP **ignore** les champs et les types qu'il ne connaît pas.
- Un changement incompatible passe à `/2` ; le RIP l'annonce dans son journal des versions
  avant de le livrer.

## 2. Identifiants

- Chaque **machine**, **couche de découpe**, **réglage enregistré**, **support** et **mode
  d'impression** porte un `uuid` (UUID v4) **attribué par le serveur et immuable**.
- Le nom affiché et l'identifiant texte d'une machine (`epson`, `kona`…) peuvent changer :
  l'ERP s'appuie sur l'`uuid`, jamais sur un nom.
- Un `uuid` inconnu envoyé au RIP est refusé (`identifiant_inconnu`) : on ne choisit pas
  son identifiant.
- Un réglage enregistré renommé devient un autre réglage (nouvel `uuid`) : son nom est
  aussi son dossier surveillé.
- Profils ICC : identifiés par leur nom de fichier (pas d'`uuid` à ce stade).

## 3. Accès machine à machine

- Le jeton de l'ERP se crée sur le serveur du RIP, par un administrateur :

  ```bash
  sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur erp jeton
  ```

  Il s'affiche **une seule fois** ; le serveur n'en garde que l'empreinte SHA-256
  (`erp.json`, droits 0600). En créer un nouveau invalide l'ancien.
  `… erp etat` dit ce qui est déclaré, sans rien révéler.
- L'ERP l'envoie dans chaque appel : `Authorization: Bearer <jeton>`.
- Le jeton **n'ouvre que** les routes `/api/erp/` :

  | Cas | Réponse |
  |---|---|
  | Route `/api/erp/…` sans jeton ou jeton faux | `401` `jeton_erp_requis` |
  | Route `/api/erp/…` avec le bon jeton | traitée |
  | Jeton présenté sur une autre route | `403` `jeton_erp_hors_pont` |

  Les routes `/api/erp/` restent fermées sans jeton **même quand l'accès humain est libre**
  (serveur sans compte).
- Erreurs : corps JSON `{ "code": "<code stable>", "detail": "<texte ou null>" }`.

Route du lot R1 : `GET /api/erp/contrat` → `{ contrat, serveur, signature, etats }`.

## 4. Signature des événements (RIP → ERP)

- Le secret partagé se crée sur le serveur du RIP et s'affiche une seule fois :

  ```bash
  sudo -u eko-rip env EKO_RIP_DATA=/var/lib/eko-rip eko-rip-serveur erp secret
  ```

- Chaque événement envoyé porte l'en-tête :

  ```
  X-EKO-RIP-Signature: t=<secondes Unix>,v1=<hex>
  ```

  avec `v1 = HMAC-SHA256(clé, message)` en minuscules hexadécimales, où **clé** = le
  secret tel qu'affiché (64 caractères, pris comme octets ASCII) et **message** =
  `"<t>." + corps`, le corps étant les **octets exacts** reçus (ne pas re-sérialiser le
  JSON).
- L'ERP **refuse** : une signature fausse (comparaison en temps constant), un `t` à plus
  de 5 minutes de son horloge (rejeu). Un `seq` déjà traité est un doublon : répondre 200
  sans rien refaire.
- Vecteur de contrôle (RFC 4231, cas 2) : clé `Jefe`, message
  `what do ya want for nothing?` →
  `5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843`.

Enveloppe d'un événement (transport livré avec le lot R2) :

```json
{
  "contrat": "eko-rip-erp/1",
  "seq": 42,
  "id": "6f1c0e8e-1b8e-4b9a-9d5e-2f6f3c1a7b10",
  "type": "sortie.etat",
  "le": 1793000000,
  "donnees": { }
}
```

- `seq` : numéro croissant **sans trou**, propre au serveur ; l'ERP peut demander à
  rejouer « tout depuis N ».
- `id` : identifiant unique de l'événement.
- `type` : `catalogue.modifie`, `travail.etat`, `sortie.etat`, `travail.consommation`…

## 5. Modèle d'états

Publié aussi, lisible par programme, dans `GET /api/erp/contrat` (`etats`).

**Travail** (le rendu, ou RIP) :

| État | Sens |
|---|---|
| `en_attente` | reçu, en attente du rendu |
| `en_cours` | rendu en cours |
| `termine` | rendu terminé : prêt à sortir |
| `echec` | rendu impossible (code d'erreur stable) |

Après une coupure : un travail `en_cours` redevient `en_attente` et son rendu reprend.

**Sortie** (l'envoi d'une page à une machine) :

| État | Sens |
|---|---|
| `en_attente` | en file vers une machine |
| `suspendue` | retenue par un opérateur |
| `en_cours` | envoi à la machine en cours |
| `terminee` | envoyée à la machine (impression ou découpe) |
| `echec` | envoi impossible (code d'erreur stable) |
| `annulee` | retirée avant envoi |

Après une coupure : une sortie `en_cours` passe en `echec` avec le code `interrompu`. Elle
ne repart pas seule : une découpe ne se reprend pas au milieu.

À venir (lot R5, pilote direct de l'Epson) : `en_impression` et `imprimee`, émis par la
machine elle-même. D'ici là, `terminee` signifie « confiée à la machine », pas « sortie
de la machine ».

Correspondance suggérée côté ERP : travail `termine` → « prêt à imprimer » ; sortie
`terminee` d'une imprimante → « en impression » ; sortie `terminee` d'une découpeuse →
« découpé » ; `echec` → « bloqué », avec le code.
