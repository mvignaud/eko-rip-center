#!/bin/sh
# Publie le site vitrine https://e-ko.rip (dossier site/) sur son hébergement.
#
#   EKO_SITE_HOTE=<compte>@<hôte o2switch> outils/publier-site.sh
#   (ou un alias de ~/.ssh/config ; EKO_SITE_DOSSIER vaut public_html par défaut)
#
# Le site est statique : rsync suffit. Le journal des versions est lu en direct sur l'ERP
# (https://app.e-ko.fr/changelog-rip.json) ; ce script en dépose aussi une copie
# (changelog.fr.json, changelog.en.json) que le site lit si l'ERP ne répond pas.
# La source du journal reste resources/changelog-rip/ d'eko-source : c'est l'ERP qui
# envoie l'e-mail aux abonnés à son déploiement. Ne rien envoyer d'ici.
set -eu

HOTE=${EKO_SITE_HOTE:?"EKO_SITE_HOTE manquant : compte SSH de l'hébergement (compte@hôte ou alias de ~/.ssh/config)"}
DOSSIER=${EKO_SITE_DOSSIER:-public_html}
ERP=${EKO_ERP_URL:-https://app.e-ko.fr}

RACINE=$(cd "$(dirname "$0")/.." && pwd)

# La documentation (site/documentation/) est générée depuis docs/ : ne jamais publier une
# version en retard sur sa source.
python3 "$RACINE/outils/generer-documentation.py" --verifier

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# Une copie du journal traînant dans site/ ne doit jamais remplacer celle en ligne (revue Graphify).
rsync -a --exclude README.md --exclude 'changelog.*.json' "$RACINE/site/" "$TMP/"

# Les pages ne sont jamais mises en cache, mais le CSS et le JS le sont une heure (.htaccess) :
# sans version dans l'URL, un téléphone garde l'ancien style.css sous une page neuve. Chaque
# publication suffixe donc les liens vers chaque .css et .js d'assets/ d'une empreinte de leur contenu (?v=…).
for chemin in "$TMP"/assets/*.css "$TMP"/assets/*.js; do
  [ -f "$chemin" ] || continue  # motif sans correspondance
  f=${chemin#"$TMP/"}
  v=$(cksum "$chemin" | cut -d' ' -f1)
  find "$TMP" -name '*.html' -exec perl -pi -e "s#\"/\Q$f\E\"#\"/$f?v=$v\"#g" {} +
done

for langue in fr en; do
  if curl -fsS --max-time 20 "$ERP/changelog-rip.json?lang=$langue" -o "$TMP/changelog.$langue.json.tmp" \
     && grep -q '"releases"' "$TMP/changelog.$langue.json.tmp"; then
    mv "$TMP/changelog.$langue.json.tmp" "$TMP/changelog.$langue.json"
  else
    rm -f "$TMP/changelog.$langue.json.tmp"
    echo "Avertissement : journal $langue indisponible sur $ERP, la copie de secours n'est pas mise à jour." >&2
  fi
done

# Droits posés ici et non par --chmod, que l'openrsync de macOS refuse. Sans -p, rsync ne
# touche pas aux droits de la racine distante (public_html en 750 chez o2switch).
chmod -R u=rwX,go=rX "$TMP"

# --delete retire les fichiers supprimés du site, mais garde la copie du journal déjà en ligne
# quand l'ERP n'a pas répondu, et les fichiers propres à l'hébergeur (.well-known, cgi-bin).
rsync -rlvz --delete \
  --filter='P changelog.*.json' --filter='P .well-known/' --filter='P cgi-bin/' \
  "$TMP/" "$HOTE:$DOSSIER/"

echo "Site publié sur $HOTE:$DOSSIER. Vérification : curl -sI https://e-ko.rip/ | head -1"
