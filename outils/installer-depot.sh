#!/bin/sh
# Ajoute le dépôt APT d'E-KO RIP Center et installe le client. Ensuite,
# « sudo apt update && sudo apt upgrade » le tient à jour.
#   wget -qO- {url}/installer | sudo sh
set -e
[ "$(id -u)" = 0 ] || { echo "À lancer en administrateur : wget -qO- {url}/installer | sudo sh" >&2; exit 1; }
CLE=/usr/share/keyrings/eko-rip.gpg
if command -v wget >/dev/null; then wget -qO "$CLE.tmp" "{url}/eko-rip.gpg"; else curl -fsSo "$CLE.tmp" "{url}/eko-rip.gpg"; fi
mv "$CLE.tmp" "$CLE"
chmod 644 "$CLE"
echo "deb [signed-by=$CLE] {url} ./" > /etc/apt/sources.list.d/eko-rip.list
apt-get update
apt-get install -y e-ko-rip-center
echo "E-KO RIP Center installé. Mises à jour : sudo apt update && sudo apt upgrade"
