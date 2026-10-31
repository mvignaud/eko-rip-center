#!/bin/sh
# Publie le site vitrine https://e-ko.rip (dossier site/) sur son hébergement.
#
#   outils/publier-site.sh                 # hôte et dossier par défaut
#   EKO_SITE_HOTE=n0c EKO_SITE_DOSSIER=e-ko.rip outils/publier-site.sh
#
# Le site est statique : rsync suffit. Le journal des versions est lu en direct sur l'ERP
# (https://app.e-ko.fr/changelog-rip.json) ; ce script en dépose aussi une copie
# (changelog.fr.json, changelog.en.json) que le site lit si l'ERP ne répond pas.
# La source du journal reste resources/changelog-rip/ d'eko-source : c'est l'ERP qui
# envoie l'e-mail aux abonnés à son déploiement. Ne rien envoyer d'ici.
set -eu

HOTE=${EKO_SITE_HOTE:-sc3nihe1770@fleur.o2switch.net}
DOSSIER=${EKO_SITE_DOSSIER:-e-ko.rip}
ERP=${EKO_ERP_URL:-https://app.e-ko.fr}

RACINE=$(cd "$(dirname "$0")/.." && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

rsync -a --exclude README.md "$RACINE/site/" "$TMP/"

for langue in fr en; do
  if curl -fsS --max-time 20 "$ERP/changelog-rip.json?lang=$langue" -o "$TMP/changelog.$langue.json.tmp" \
     && grep -q '"releases"' "$TMP/changelog.$langue.json.tmp"; then
    mv "$TMP/changelog.$langue.json.tmp" "$TMP/changelog.$langue.json"
  else
    rm -f "$TMP/changelog.$langue.json.tmp"
    echo "Avertissement : journal $langue indisponible sur $ERP, la copie de secours n'est pas mise à jour." >&2
  fi
done

# --delete retire les fichiers supprimés du site, mais garde la copie du journal déjà en ligne
# quand l'ERP n'a pas répondu, et les fichiers propres à l'hébergeur (.well-known, cgi-bin).
rsync -rlvz --delete --chmod=D755,F644 \
  --filter='P changelog.*.json' --filter='P .well-known/' --filter='P cgi-bin/' \
  "$TMP/" "$HOTE:$DOSSIER/"

echo "Site publié sur $HOTE:$DOSSIER. Vérification : curl -sI https://e-ko.rip/ | head -1"
