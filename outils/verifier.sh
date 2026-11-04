#!/usr/bin/env bash
# Les contrôles de la CI (.github/workflows/ci.yml), à lancer en local avant de
# pousser : le compte n'a plus de minutes GitHub Actions. Demande Rust, Node,
# Python 3 et Ghostscript. Ne construit pas le .deb (cargo deb), trop long ici.
set -euo pipefail
cd "$(dirname "$0")/.."

command -v "${EKO_RIP_GS:-gs}" >/dev/null || { echo "Ghostscript introuvable : les tests de rendu échoueraient (apt install ghostscript)"; exit 1; }

echo "· syntaxe JavaScript des pages"
for f in serveur/static/index.html client/ui/index.html; do
  python3 -c "import re,sys; print(re.search(r'<script>(.*)</script>', open(sys.argv[1]).read(), re.S).group(1))" "$f" > /tmp/eko-rip-page.js
  node --check /tmp/eko-rip-page.js || { echo "erreur de syntaxe dans $f"; exit 1; }
done

echo "· documentation publique à jour (docs/ → site/documentation/)"
python3 outils/generer-documentation.py --verifier

echo "· site à jour (blog/, pages/, menu commun → site/)"
python3 outils/generer-site.py --verifier

echo "· articles du centre d'aide (docs/aide/, sans réseau)"
python3 outils/publier-aide --verifier

echo "· clés de traduction en double"
node outils/doublons-traductions.js serveur/static/index.html

echo "· clippy"
cargo clippy -p eko-rip-serveur -p eko-rip-cloud --all-targets --locked -- -D warnings

echo "· tests"
cargo test -p eko-rip-serveur -p eko-rip-cloud --locked

echo "Tout est vert."
