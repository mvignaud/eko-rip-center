# Blog e-ko.rip

Source des articles de https://e-ko.rip/blog/. Un fichier Markdown par article dans `fr/`
(nom du fichier = adresse : `fr/choisir-son-rip.md` → `/blog/choisir-son-rip/`).

```
---
titre: Titre de l'article (60 caractères environ)
description: Résumé pour Google et les réseaux (140 à 160 caractères)
date: 2026-11-03
categorie: Guides
image: /assets/img/linux-travaux.webp
alt: Texte de remplacement de l'image
---
Chapeau (premier paragraphe), puis le corps en Markdown : ## et ### titres, listes,
tableaux, citations, encadrés > [!TIP], liens.

## Questions fréquentes

### Une question ?

La réponse (devient aussi des données structurées FAQPage).
```

Catégories : Guides, Couleur, Atelier, Logiciel.

`python3 outils/generer-site.py` écrit `site/blog/` (pages, index, flux RSS) et le plan du site ;
`--verifier` échoue s'il est en retard (dans `outils/verifier.sh` et la CI).
