# Pont ERP ↔ RIP — contrat `eko-rip-erp/1` (lots R1, R2 et R3)

Ce document fixe ce qu'un ERP peut attendre d'E-KO RIP Center. Le lot R1 pose les
fondations : identifiants, accès machine à machine, signature des événements, modèle
d'états ; le lot R2 (§ 6) le canal : journal, envois, instances, catalogue, états et
consommation. Les lots suivants (R2 canal et catalogue, R3 réception des travaux, R4 estimation,
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

Enveloppe d'un événement (transport : § 6) :

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

- `instance` : identifiant (UUID) de cette installation du pont, tiré au premier démarrage,
  gardé dans `erp.json` et repris par la sauvegarde de la configuration (ajout compatible du
  25/10, adopté par l'ERP E-KO).
- `seq` : numéro croissant **sans trou** pour une instance ; l'ERP dédoublonne et détecte les
  trous sur le couple `(instance, seq)`. Une instance inconnue qui commence à `seq = 1` ferme
  la précédente ; une instance inconnue avec `seq > 1` reçoit 409 `seq_manquant` et
  `dernier_seq_traite = 0` (le RIP rejoue depuis 1). Si l'ERP répond, pour la MÊME instance,
  un `dernier_seq_traite` plus grand que le dernier `seq` du RIP, le RIP cesse d'envoyer et le
  signale dans Système : il ne renumérote jamais en silence.
- `id` : identifiant unique de l'événement.
- `type` : `catalogue.modifie`, `travail.etat`, `sortie.etat`, `travail.consommation`, `machine.etat`…

## 5. Modèle d'états

Publié aussi, lisible par programme, dans `GET /api/erp/contrat` (`etats`).

**Travail** (le rendu, ou RIP) :

| État | Sens |
|---|---|
| `en_attente` | reçu, en attente du rendu |
| `en_cours` | rendu en cours |
| `termine` | rendu terminé : prêt à sortir |
| `echec` | rendu impossible (code d'erreur stable) |

Après une coupure : un travail `en_cours` passe en `echec` avec le code `interrompu` ; il est
à relancer (il ne repart pas seul).

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

`imbriquee` (ajout compatible, lot R2) : la sortie est posée sur une planche d'imbrication ;
la planche, un travail à part, l'imprime, et la sortie passe `terminee` quand la planche
est imprimée. La consommation est alors portée par la planche.

À venir (lot R5, pilote direct de l'Epson) : `en_impression` et `imprimee`, émis par la
machine elle-même. D'ici là, `terminee` signifie « confiée à la machine », pas « sortie
de la machine ».

Correspondance suggérée côté ERP : travail `termine` → « prêt à imprimer » ; sortie
`terminee` d'une imprimante → « en impression » ; sortie `terminee` d'une découpeuse →
« découpé » ; `echec` → « bloqué », avec le code.


## 6. Canal et événements (lot R2, côté RIP)

### 6.1 Réglage

Dans le client, **Système > Pont ERP** (administrateur) : URL de base de l'ERP, secret HMAC,
activation. L'URL est celle que donne l'écran du pont de l'ERP
(`https://<hôte>/api/v1/gestion-imprimerie/rip/<atelier>`) : `https` obligatoire (`http`
n'est admis que vers la machine elle-même, pour les essais). Le secret se crée avec
`eko-rip-serveur erp secret` ou se colle dans l'écran ; il n'est **jamais** réaffiché ni
renvoyé par l'API, ni écrit dans la sauvegarde de la configuration (qui reprend l'URL,
l'activation et l'instance). L'écran montre l'état (désactivé, actif, en erreur, remplacé,
anomalie), le dernier `seq` numéroté et le dernier accusé par l'ERP, le dernier contact, la
dernière erreur, et l'instance, avec **Tirer une nouvelle instance** (confirmation).

### 6.2 Journal

- Tant que le pont est activé, chaque événement est écrit dans `pont/journal.jsonl` (une
  ligne, synchronisée sur le disque) **avant** tout envoi ; `seq` croît sans trou pour
  l'instance. Une coupure de courant pendant l'écriture laisse au plus une ligne coupée :
  elle est retirée au redémarrage, rien d'autre ne change (aucun événement perdu, aucun
  renuméroté).
- L'état du pont (`pont/etat.json`) garde le dernier accusé et le dernier état publié de
  chaque travail et de chaque sortie.
- Les événements accusés sont purgés après 30 jours ; ceux qui ne le sont pas restent
  jusqu'à leur accusé, quelle que soit la durée de la panne.
- Pont désactivé : rien n'est journalisé. À l'activation, les travaux et sorties existants
  sont pris tels quels ; seuls leurs changements suivants font un événement.

### 6.3 Échanges

- `GET <base>/en-attente?instance=<uuid>`, toujours avec l'instance, signé sur
  `"<t>.GET <chemin>?instance=<uuid>"` ; toutes les 30 s, toutes les 5 s quand une demande
  attend (`catalogue_demande`), et aussitôt qu'un événement s'ajoute.
- Puis `POST <base>/evenements` : une **liste** de 100 événements au plus, dans l'ordre de
  `seq`, signée sur le corps exact ; tout ce qui suit le dernier accusé.
- Débit : **une requête par seconde au plus** (GET compris), soit 60 par minute même en
  rafale, sous la limite de 120 de l'ERP ; les événements arrivés pendant l'attente partent
  dans le lot suivant. Sur `429`, le RIP attend `Retry-After` (en secondes, 1 h au plus),
  sinon 60 s, avant tout nouvel appel.
- Réponses : `200` et `409 seq_manquant` donnent `dernier_seq_traite` : le RIP renvoie tout
  ce qui suit. Toute autre erreur (401, 403, 404, 422, 429, 5xx, réseau) est affichée dans
  Système avec son code ; le RIP réessaie après 60 s, puis toutes les 5 min. Il ne saute
  jamais un événement refusé.
- `travaux` : les travaux envoyés par l'ERP (lot R3, § 7).
- `annulations`, `estimations` : lus, ignorés tant que leur forme n'est pas fixée (lots R4,
  R5) ; le journal du serveur le dit (« non prises en charge »).

### 6.4 Instances

- L'instance (UUID v4) est tirée au premier démarrage du serveur et gardée dans `erp.json`.
- Réponse `dernier_seq_traite = 0` pour une instance **jamais accusée** : tout est rejoué
  depuis `seq = 1` (installation neuve).
- Réponse `0` pour une instance **déjà accusée** : une autre installation a pris la place.
  Le RIP n'envoie plus rien (ni `GET` ni `POST`) et affiche « remplacé » dans Système. Seul
  l'administrateur le relance, en tirant une nouvelle instance : le journal repart de 1.
- Réponse plus grande que le dernier `seq` du RIP (même instance), ou rejeu demandé
  d'événements purgés : « anomalie », arrêt, jamais de renumérotation silencieuse. Après
  vérification avec l'ERP : nouvelle instance.
- **Remise à zéro de la séquence depuis l'écran de l'ERP** : l'ERP répond alors 0 pour une
  instance déjà accusée, et le RIP ne peut pas la distinguer d'un remplacement. Il passe
  « remplacé » et s'arrête ; il faut tirer une nouvelle instance dans Système pour qu'il
  reprenne (journal rejoué depuis 1).

### 6.5 Événements

`catalogue.modifie` — catalogue complet, au démarrage du pont, à chaque modification
(machines, couches de découpe, supports, modes, réglages enregistrés, profils ICC) et quand
l'ERP le demande. Clés : `machines` (imprimantes), `decoupeuses`, `couches_decoupe`,
`supports`, `modes`, `prereglages`, `profils`, et `serveur` (version). Chaque objet porte son
`uuid` (sauf les profils, identifiés par `fichier`) et son `nom` ; `machine_uuid`,
`support_uuid`, `mode_uuid`, `decoupeuse_uuid` les relient. Un champ inconnu est omis, jamais
`null`. Aucun prix.

`travail.etat` — à chaque changement d'état d'un travail :

```json
{ "travail_uuid": "…", "ref_erp": null, "etat": "termine", "code_erreur": null, "pages": 2 }
```

Un travail né d'un envoi de l'ERP (§ 7) porte en plus `envoi_id`, et sa référence :

```json
{ "travail_uuid": "…", "ref_erp": "PROD-2026-00123", "envoi_id": "…", "etat": "en_attente", "code_erreur": null, "pages": 0 }
```

`sortie.etat` — à chaque changement d'état d'une sortie :

```json
{ "sortie_uuid": "…", "travail_uuid": "…", "ref_erp": null, "page": 1, "machine_uuid": "…",
  "machine": "epson", "operation": "impression", "etat": "en_attente", "code_erreur": null,
  "raison_attente": "attente_support", "planche_uuid": null }
```

`travail.consommation` — à la fin d'une impression, puis d'une découpe (sortie `terminee`) :

```json
{ "travail_uuid": "…", "sortie_uuid": "…", "ref_erp": null, "page": 1, "operation": "impression",
  "machine_uuid": "…", "machine": "epson", "support_uuid": "…", "mode_uuid": "…", "couche": null,
  "copies": 1, "surface_m2": 1.2, "longueur_prevue_mm": 1000, "longueur_mm": 1204,
  "longueur_source": "reel", "gache_mm": 204, "encre_ml": { "C": 3.5 }, "encre_sous_seuil_ml": null,
  "encre_ml_estimee": { "C": 3.1 }, "encre_source": "reel", "duree_s": 600, "debut": 1793000000, "fin": 1793000600 }
```

- Rien n'est inventé : sans relevé de la machine (SNMP), `encre_ml`, `longueur_mm` et
  `gache_mm` valent `null` ; `encre_source` dit d'où vient l'encre donnée (`reel` mesuré,
  `estime` par les points posés du pilote direct, `null` : aucune).
- `encre_ml` est l'encre réelle répartie au moment de la fin (la répartition d'une baisse
  entre plusieurs impressions peut s'affiner ensuite dans les rapports du RIP).
- `ref_erp` : la référence du dossier pour un travail envoyé par l'ERP (§ 7), recopiée dans
  ses sorties et ses consommations ; `null` pour un travail créé au RIP.

`machine.etat` (roadmap 7.9) — état relevé d'une imprimante (SNMP, toutes les minutes), publié
seulement quand il change (l'heure seule ne compte pas) :

```json
{ "machine_uuid": "…", "machine": "epson", "encres": [{ "encre": "BK", "pourcent": 80 }],
  "longueur_restante_m": 37.5, "support_uuid": "…", "laize_mm": 1620,
  "chauffages": [{ "ecran_c": 40, "autre_c": 45 }], "alertes": ["printer cover open"],
  "bac_encre_usagee_pourcent": null, "releve_le": 1793000000 }
```

- `encres` : % restant, au point près comme l'imprimante le donne ; `longueur_restante_m` :
  longueur saisie au chargement du rouleau, décomptée par la machine (`null` si inconnue).
- `support_uuid` et `laize_mm` : support chargé (saisi par l'opérateur ou lu sur la machine).
- `chauffages` : deux valeurs par chauffage ; `ecran_c` est celle de l'écran de l'imprimante.
- `alertes` : textes de la machine, tels quels, bloquants ou non.
- `bac_encre_usagee_pourcent` : `null` tant qu'il n'est pas lu sur l'Epson.
- Aucun envoi à la demande pour l'instant : l'ERP garde le dernier `machine.etat` reçu.

## 7. Travaux envoyés par l'ERP (lot R3)

Forme d'un élément de `travaux` : § 11 du contrat côté ERP (`docs/onyx/pont-erp-cote-erp.md`
d'eko-source) — `id` (uuid de l'envoi), `ref_erp`, `nom`, `prereglage_uuid`, `copies`,
`echeance`, `fichier { chemin, nom, taille, sha256 }`.

- **Téléchargement** : à chaque `en-attente`, chaque envoi jamais pris est téléchargé par
  `GET <base><fichier.chemin>`, signé comme `en-attente`, 5 par cycle au plus, 4 Go au plus.
  Le fichier est vérifié par son `sha256` quand l'ERP le donne, puis gardé dans
  `<données>/pont/recus/<id>/` jusqu'à devenir un travail.
- **Création** : le travail prend l'uuid de l'envoi, le réglage enregistré désigné par
  `prereglage_uuid`, et sa fiche : commande = `ref_erp`, quantité = `copies`, échéance. Un
  serveur coupé entre la création et le ménage ne refait pas de second travail.
- **Accusé** : le premier `travail.etat` du travail porte `envoi_id` (§ 6.5).
- **Refus** : `travail.refuse` — `{ "envoi_id", "ref_erp", "code", "detail" }`. Codes :
  `envoi_invalide` (élément mal formé ; `detail` nomme le champ), `telechargement_impossible`
  (404), `empreinte_fausse`, `reglage_inconnu`, et les refus de création d'un travail
  (`format_inconnu`, `pdf_illisible`, `polices_manquantes`…).
- **Doublons** : un envoi pris ou refusé n'est jamais repris, même reproposé ; il est
  oublié 30 jours après que l'ERP a cessé de le proposer. Une panne (réseau, 5xx, disque)
  n'est pas un refus : l'envoi est repris au cycle suivant.
