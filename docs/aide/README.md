# Articles du centre d'aide (catégories E-KO RIP et E-KO RIP Cloud)

Source des articles publiés dans le centre d'aide public de la Suite E-KO
(`https://help.suite-eko.com/hc/aide/fr`), catégories maîtres `rip` et `rip-cloud`. La
documentation détaillée reste sur https://e-ko.rip/documentation/ (`docs/guide/`) : les articles
d'ici sont courts, orientés tâche, et renvoient vers elle quand une page détaillée existe.

## Fichiers

- `rubriques.tsv` : les sous-rubriques (slug, catégorie maître, position, nom, description).
- `<sous-rubrique>/<slug>.md` : un article. Le nom du fichier est le slug de l'article, unique dans
  tout le centre d'aide, toujours préfixé par le produit (`rip-…` ou `rip-cloud-…`).
- `images/` : les captures (PNG). Un article y renvoie par `![texte qui décrit l'écran](../images/<nom>.png)`.
- `televersements.json` : empreinte SHA-256 d'une image → `file_url` déjà envoyé (tenu par le script).

En-tête de chaque article :

```
---
titre: Installer le serveur RIP
description: Une phrase de 160 caractères au plus, reprise pour la recherche.
sous_rubrique: rip-premiers-pas
position: 2
mots_cles: installation, ubuntu, apt
---
```

Liens entre articles : `[Installer un client](../rip-premiers-pas/rip-installer-un-client.md)` ; le
script les transforme en adresses du centre d'aide.

## Style commun

- **Titre** : une action à l'infinitif (« Installer le serveur RIP »), sans question ni nom de produit en tête.
- **Description** : une phrase, 160 caractères au plus.
- **Plan** : deux lignes d'introduction (à quoi ça sert, pour qui) ; `## Avant de commencer`
  (droits, prérequis, durée) ; `## Étapes` (liste numérotée, une action par étape, libellés de
  l'interface en gras exactement comme à l'écran) ; une capture après chaque étape clé (5 au plus) ;
  `## Bon à savoir` (limites, cas particuliers, erreurs fréquentes et leur solution) ;
  `## Voir aussi` (2 à 4 liens vers d'autres articles, plus la page détaillée d'e-ko.rip s'il y en a une).
- **Ton** : vouvoiement, phrases courtes, voix active, pas de jargon interne. Un article = une tâche
  (scinder au-delà de 10 étapes).
- **Captures** : données fictives uniquement, thème clair, PNG, texte alternatif qui décrit l'écran.
- **Interdits** : aucun nom d'hébergeur, d'outil interne ou de logiciel tiers du support, aucun nom
  de concurrent, aucune date promise, aucun secret, aucune adresse IP.
- Deux langues : le français (`docs/aide/<sous-rubrique>/`) et l'anglais (`docs/aide/en/`).

## Version anglaise

`docs/aide/en/` reprend l'arborescence française : même dossier et même nom de fichier pour chaque
article (les liens entre articles restent donc identiques), sous-rubriques dans `en/rubriques.tsv`
(mêmes slugs qu'en français, une catégorie étant unique par langue). L'en-tête ajoute `slug`, le slug
anglais (unique dans tout le centre d'aide, préfixé `rip-` ou `rip-cloud-`), et `traduction_de`, le
nom de l'article français, qui relie les deux versions dans le centre d'aide. Sections : `## Before
you start`, `## Steps`, `## Good to know`, `## See also`. L'interface du RIP n'existe qu'en
français : un libellé d'écran s'écrit en anglais en gras puis, à sa première mention, en français
en italique entre parenthèses (**Print** (*Imprimer*)). Les captures sont communes aux deux langues
(`../../images/`). Un article français modifié ou ajouté l'est aussi en anglais, dans la même PR.

## Ancienne documentation d'e-ko.rip

Les pages de https://e-ko.rip/documentation/ (FR et EN) renvoient en 301 vers les articles qui les
couvrent (`docs/aide/couverture/*.tsv`, ligne `*`), grâce au bloc que `outils/redirections-documentation`
écrit dans `site/.htaccess` ; `--verifier` (dans `outils/verifier.sh`) échoue s'il est en retard. Le
bloc est mis en ligne par la publication du site.

## Publier

`python3 outils/publier-aide --verifier` (dans `outils/verifier.sh` et la CI) contrôle les en-têtes,
les slugs, les images et les liens, sans réseau. La publication part du workflow `publier-aide.yml`
(jeton `CHATWOOT_DOC_TOKEN` de l'environnement `production`) : toujours en brouillon ; le passage en
`published` (`--publier`, entrée `publier` du workflow) se fait seulement sur l'ordre de Mathieu
(« Publie l'aide ») ; un article déjà en ligne n'est modifié qu'à ce moment-là. Le script ne supprime
rien : un article retiré du dépôt passe en `archived`. Il refuse un jeton qui n'est pas celui du compte
« Documentation Suite E-KO » (auteur 4) et des catégories maîtres d'id inattendu (rip 3, rip-cloud 8).
Après un envoi, commiter `televersements.json`, rendu en artefact par le workflow.
