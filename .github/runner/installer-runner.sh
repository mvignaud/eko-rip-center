#!/usr/bin/env bash
# Installe le runner GitHub Actions du RIP sur le serveur dédié à la CI.
#
# 2026-11-03 — Plus de minutes GitHub Actions, et Mathieu veut que tout parte
# du serveur CI (OVH, 15.235.86.71, Debian 12) : CI, release, dépôt APT,
# plate-forme Cloud et site. Les jobs tournent dans des conteneurs Docker
# (Ubuntu 22.04 pour le serveur : glibc ancienne, le paquet tourne partout ;
# Ubuntu 26.04 pour le client GTK 4, qui demande libadwaita 1.4, absente de
# Debian 12).
#
# À lancer en root sur le serveur CI :
#   sudo ./installer-runner.sh <jeton>
# Jeton d'enregistrement (valable une heure), depuis le Mac :
#   gh api -X POST repos/mvignaud/eko-rip-center/actions/runners/registration-token --jq .token
#
# Dépôt PUBLIC : un job lancé par une PR venue d'un fork exécuterait du code
# inconnu ici. Les workflows ne tournent donc que sur `main`, sur une PR
# ouverte depuis ce dépôt ou à la main (condition sur chaque job). Le runner a
# son propre compte (`ci-rip`), séparé du compte `ci` de l'ERP. Il est dans le
# groupe docker, donc proche de root : ne jamais lever ces conditions.
set -euo pipefail

JETON="${1:-}"
DEPOT="https://github.com/mvignaud/eko-rip-center"
COMPTE="ci-rip"
DOSSIER="/home/${COMPTE}/actions-runner"

[ "$(id -u)" = 0 ] || { echo "À lancer en root (sudo)." >&2; exit 1; }
. /etc/os-release

echo "== Docker"
if ! command -v docker >/dev/null; then
    apt-get update -q
    apt-get install -yq ca-certificates curl gnupg jq
    install -m 0755 -d /etc/apt/keyrings
    curl -fsSL "https://download.docker.com/linux/${ID}/gpg" -o /etc/apt/keyrings/docker.asc
    echo "deb [arch=amd64 signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/${ID} ${VERSION_CODENAME} stable" \
        > /etc/apt/sources.list.d/docker.list
    apt-get update -q
    apt-get install -yq docker-ce docker-ce-cli containerd.io
fi
systemctl enable --now docker

echo "== Compte ${COMPTE}"
id "${COMPTE}" >/dev/null 2>&1 || useradd --create-home --shell /bin/bash "${COMPTE}"
usermod -aG docker "${COMPTE}"

if [ -f "${DOSSIER}/.runner" ]; then
    echo "== Runner déjà enregistré."
else
    [ -n "${JETON}" ] || { echo "Jeton d'enregistrement manquant (voir l'en-tête)." >&2; exit 1; }
    VERSION="$(curl -fsSL https://api.github.com/repos/actions/runner/releases/latest | jq -r .tag_name | sed 's/^v//')"
    sudo -u "${COMPTE}" bash -c "
        set -euo pipefail
        mkdir -p '${DOSSIER}' && cd '${DOSSIER}'
        curl -fsSL -o runner.tgz 'https://github.com/actions/runner/releases/download/v${VERSION}/actions-runner-linux-x64-${VERSION}.tar.gz'
        tar xzf runner.tgz && rm runner.tgz
        ./config.sh --unattended --url '${DEPOT}' --token '${JETON}' \
            --name \"\$(hostname)-eko-ci-rip\" --labels eko-ci-rip --replace
    "
    cd "${DOSSIER}"
    ./svc.sh install "${COMPTE}"
fi

SERVICE="$(cat "${DOSSIER}/.service")"
# Même tranche que les runners de l'ERP (plafond commun à 90 % de la RAM).
if [ -f /etc/systemd/system/eko-ci.slice ]; then
    mkdir -p "/etc/systemd/system/${SERVICE}.d"
    printf '[Service]\nSlice=eko-ci.slice\nPrivateTmp=yes\n' > "/etc/systemd/system/${SERVICE}.d/eko-ci-limites.conf"
    systemctl daemon-reload
fi
systemctl restart "${SERVICE}"

echo
echo "Runner du RIP en service : ${DEPOT}/settings/actions/runners"
docker --version
sudo -u "${COMPTE}" docker run --rm hello-world >/dev/null && echo "Docker utilisable par ${COMPTE} : ok"
systemctl is-active "${SERVICE}"
