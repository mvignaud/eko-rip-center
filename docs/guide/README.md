# Documentation publique

Source de la documentation publiée sur **https://e-ko.rip/documentation/** (français) et
**https://e-ko.rip/documentation/en/** (anglais).

| Dossier | Contenu |
|---|---|
| `fr/`, `en/` | pages du guide (prise en main, serveur, clients, application mobile, Cloud, administration, dépannage) et `sommaire.tsv` de chaque langue |
| `images/` | captures et schémas, partagés par les deux langues (`*.en.svg` : schémas traduits) |
| `../fr/`, `../en/` | pages de référence reprises telles quelles (utilisation, machines, glossaire) |

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
| Site e-ko.rip (menu et pied de page) | `/documentation/` |

Ne pas renommer une page ni un titre cité ici sans mettre à jour ces liens.
