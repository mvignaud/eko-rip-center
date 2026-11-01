## Ce qui change

<!-- En deux ou trois phrases : le besoin, ce que fait la PR. -->

## Documentation

La documentation publique (https://e-ko.rip/documentation/) suit chaque changement du RIP.
Cocher ce qui s'applique :

- [ ] Rien de visible pour l'utilisateur (refactorisation, tests, CI) : pas de doc à changer.
- [ ] Guide mis à jour en français **et** en anglais (`docs/guide/fr/`, `docs/guide/en/`).
- [ ] Références mises à jour (`docs/fr/`, `docs/en/` : utilisation, machines, glossaire, installation…).
- [ ] Captures ou schémas refaits si l'écran a changé (`docs/guide/images/`).
- [ ] `python3 outils/generer-documentation.py` lancé, `site/documentation/` commité.
- [ ] `CHANGELOG.md` complété.

## Vérifications

- [ ] `./outils/verifier.sh` vert (comprend la vérification de la documentation).
- [ ] Revues Codex **et** Graphify lues et traitées.
