#!/bin/sh
# Publie des paquets .deb (client Linux, serveur) dans le dépôt APT en ligne
# et le re-signe. Sans argument : re-signe et renvoie seulement.
#   outils/publier-depot.sh <paquet.deb>...
#
# À lancer sur une machine Linux (le serveur de test) : il faut apt-utils,
# gnupg et rsync. La clé de signature reste sur cette machine ; le VPS du
# dépôt ne reçoit que des fichiers déjà signés.
set -e
URL=${EKO_DEPOT_URL:-https://apt.e-ko.fr/rip}
DISTANT=${EKO_DEPOT_RSYNC:-depot@apt.e-ko.fr:rip/}
LOCAL=${EKO_DEPOT_LOCAL:-$HOME/.eko-rip-depot/rip}
CLE=${EKO_DEPOT_CLE:-depot@e-ko.fr}
GARDER=${EKO_DEPOT_GARDER:-5}
ICI=$(cd "$(dirname "$0")" && pwd)
for o in apt-ftparchive dpkg-deb gpg rsync; do
    command -v $o >/dev/null || { echo "Manque $o (apt install apt-utils gnupg rsync)" >&2; exit 1; }
done
mkdir -p "$LOCAL"

if ! gpg --batch --list-secret-keys "$CLE" >/dev/null 2>&1; then
    gpg --batch --quiet --pinentry-mode loopback --passphrase '' \
        --quick-generate-key "Dépôt E-KO RIP Center <$CLE>" ed25519 sign never
    echo "Clé de signature créée. À SAUVEGARDER hors de cette machine (sans elle, les postes"
    echo "refuseront les prochaines mises à jour) : gpg --export-secret-keys $CLE > cle-depot.gpg"
fi
gpg --batch --yes --export "$CLE" > "$LOCAL/eko-rip.gpg"
sed "s|{url}|$URL|g" "$ICI/installer-depot.sh" > "$LOCAL/installer"

for f in "$@"; do
    case "$f" in *.deb) ;; *) echo "Pas un .deb : $f" >&2; exit 1 ;; esac
    paquet=$(dpkg-deb -f "$f" Package)
    version=$(dpkg-deb -f "$f" Version)
    arch=$(dpkg-deb -f "$f" Architecture)
    install -m 0644 "$f" "$LOCAL/${paquet}_${version}_${arch}.deb"
    echo "Ajouté : $paquet $version"
    # Ne garder que les dernières versions de chaque paquet.
    ls "$LOCAL/${paquet}_"*"_${arch}.deb" | sort -V | head -n "-$GARDER" | xargs -r rm -f
done

(
    cd "$LOCAL"
    apt-ftparchive packages . > Packages
    gzip -9kf Packages
    apt-ftparchive -o APT::FTPArchive::Release::Origin="E-KO RIP Center" \
        -o APT::FTPArchive::Release::Label="E-KO RIP Center" release . > Release
    gpg --batch --yes --local-user "$CLE" --clearsign -o InRelease Release
    gpg --batch --yes --local-user "$CLE" -abs -o Release.gpg Release
)

# Les paquets d'abord, les index ensuite : un poste qui lit le dépôt pendant
# l'envoi ne voit jamais un index qui annonce un paquet absent.
rsync -rt --chmod=F644,D755 --include='*.deb' --exclude='*' "$LOCAL/" "$DISTANT"
rsync -rt --chmod=F644,D755 --delete-after "$LOCAL/" "$DISTANT"
echo "Dépôt publié : $URL ($(grep -c '^Package:' "$LOCAL/Packages") paquet(s))."
