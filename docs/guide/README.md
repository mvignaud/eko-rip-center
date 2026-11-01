# Documentation publique

Source de la documentation publiée sur **https://e-ko.rip/documentation/** (français) et
**https://e-ko.rip/documentation/en/** (anglais).

| Dossier | Contenu |
|---|---|
| `fr/`, `en/` | pages du guide (prise en main, serveur, clients, application mobile, Cloud, administration, dépannage) et `sommaire.tsv` de chaque langue |
| `images/` | captures et schémas, partagés par les deux langues (`*.en.svg` : schémas traduits) |
| `../fr/`, `../en/` | pages de référence reprises telles quelles (utilisation, glossaire) et rubrique Machines (`machines.md` + une page par modèle dans `machines/`) |

`outils/generer-documentation.py` (Python 3, sans dépendance) convertit le Markdown en pages
HTML dans `site/documentation/`, au style du site, avec sommaire, recherche, « Sur cette page »
et sélecteur de langue. Les pages générées sont commitées : le site reste statique.

## Règle : la documentation suit chaque changement

Toute pull request qui change ce que voit ou fait un utilisateur (serveur, interface
d'administration, clients Mac, Linux et Windows, application mobile, Cloud, installation)
met à jour la documentation **dans la même pull request**, dans les deux langues :

1. modifier la page concernée dans `fr/` et `en/` (ou `docs/fr/` et `docs/en/` pour les
   références) ; une nouvelle page s'ajoute dans les deux `sommaire.tsv` avec le même
   identifiant ;
2. refaire la capture si l'écran a changé (même nom de fichier, PNG réduit avec `pngquant`) ;
3. lancer `python3 outils/generer-documentation.py` et committer `site/documentation/`.

`python3 outils/generer-documentation.py --verifier` échoue si `site/documentation/` est en
retard sur `docs/` : il tourne dans `outils/verifier.sh`, dans la CI et avant chaque
`outils/publier-site.sh`. La documentation est en ligne quand le site est publié.

## Machines : une page par modèle

La rubrique **Machines** range les modèles par famille (« === Imprimantes », « === Plotters de
découpe », « === Tables de découpe » dans `sommaire.tsv`). Chaque modèle de l'enum `Modele`
de `serveur/src/machines.rs` a sa page, dans les deux langues, avec l'identifiant tiré du nom
de la variante (`EpsonScS80600` → `epson-sc-s80600`, adresse `machines/epson-sc-s80600/`) :
le générateur échoue sinon. Ajouter un modèle au RIP, c'est donc aussi :

1. écrire `docs/fr/machines/<identifiant>.md` et `docs/en/machines/<identifiant>.md` (mêmes
   sections que les autres : ce que fait le RIP, brancher, sur la machine, dans le RIP,
   vérifier, ennuis courants) ;
2. l'ajouter dans les deux `sommaire.tsv`, sous sa famille (une famille nouvelle s'ouvre par
   une ligne « === ») ;
3. l'ajouter au tableau de `docs/fr/machines.md` et `docs/en/machines.md`.

Le lien **Aide** à côté du modèle, dans Administration › Système › Machines, mène à cette page.

## Écrire

- Public : imprimeurs et opérateurs, pas développeurs. Phrases courtes, consignes à
  l'infinitif en français (« Ouvrir… »), à l'impératif en anglais.
- Libellés de l'interface en **gras**, exactement comme à l'écran.
- Pas de `# Titre` en tête de page : le titre et le chapeau viennent de `sommaire.tsv`.
- Encadrés : `> [!TIP]`, `> [!NOTE]`, `> [!WARNING]`.
- Image seule sur sa ligne : `![texte alternatif](../images/nom.png "Légende")`.
- Liens entre pages en relatif vers le `.md` (`serveur.md#créer-les-comptes`) : le
  générateur les transforme en adresses du site. Les ancres suivent les titres (règle GitHub).
- Ne jamais citer d'autre RIP du marché ni de détail d'hébergement interne.

## Liens « Aide » dans les logiciels

| Où | Lien |
|---|---|
| Interface du serveur (barre du haut, bouton **Aide**) | `/documentation/` ou `/documentation/en/` selon la langue |
| Page d'état du serveur (navigateur) | `/documentation/clients/` |
| Clients Mac et Linux (menu Aide) | `/documentation/clients/#client-mac`, `#client-linux` |
| Client Windows | `/documentation/clients/#client-windows` |
| Application mobile (Réglages) | `/documentation/application-mobile/` |
| Portail Cloud (`platform.e-ko.rip`) | `/documentation/cloud/` |
| Interface du serveur, Système › Machines (lien **Aide** du modèle) | `/documentation/machines/<modèle>/` (ou `/en/…`) |
| Site e-ko.rip (menu et pied de page) | `/documentation/` |

Ne pas renommer une page ni un titre cité ici sans mettre à jour ces liens.
