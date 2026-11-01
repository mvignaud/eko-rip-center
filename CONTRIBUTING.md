# Contribuer à E-KO RIP Center

Merci de votre intérêt. Le projet est publié sous
[AGPL-3.0-or-later](LICENSE) par GOLDEN PRINT.

## Accord de contribution

GOLDEN PRINT reste titulaire des droits sur l'ensemble du code, ce qui lui permet de
proposer aussi le logiciel sous licence commerciale (double licence). Pour qu'une
contribution soit acceptée, son auteur accepte, en ouvrant la pull request, que :

1. la contribution est son œuvre originale, ou il a le droit de la proposer ;
2. il accorde à GOLDEN PRINT une licence perpétuelle, mondiale, gratuite, non exclusive
   et irrévocable d'utiliser, modifier, sous-licencier et distribuer la contribution,
   sous l'AGPL-3.0-or-later comme sous toute autre licence, y compris commerciale ;
3. la contribution est publiée dans le dépôt sous AGPL-3.0-or-later.

Chaque commit porte la ligne `Signed-off-by: Nom <e-mail>` (`git commit -s`), qui vaut
acceptation de cet accord.

## Avant d'ouvrir une pull request

- Lancer `./outils/verifier.sh` (syntaxe des pages, traductions, clippy, tests).
- Ne pas lancer `cargo fmt` : le projet n'utilise pas rustfmt.
- Toute nouvelle dépendance doit avoir une licence compatible avec l'AGPL-3.0
  (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, LGPL…) ; les profils ICC, polices ou
  pilotes ajoutés doivent pouvoir être redistribués.
- Ne jamais committer de secret (jeton, clé, mot de passe).
