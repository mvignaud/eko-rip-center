#!/usr/bin/env bash
# Construit l'image d'installation d'E-KO RIP Center à partir de l'ISO officielle
# Ubuntu 24.04 LTS Server. L'ISO d'origine est modifiée par xorriso, qui rejoue son
# démarrage BIOS et UEFI tel quel : seul le menu GRUB change, et le dossier /eko
# (profils d'installation automatique + paquets serveur et client) s'ajoute.
#
# Trois profils au démarrage :
#   - serveur + client : bureau Ubuntu, serveur RIP, client ouvert au démarrage sur ce PC
#     (petit atelier : un seul PC) ;
#   - serveur + bureau : bureau Ubuntu et serveur RIP (PC d'usage, clients ailleurs) ;
#   les deux profils de bureau ajoutent CUPS et la suite ArtCraft (téléchargée) ;
#   - serveur seul : sans bureau.
# L'installateur ne demande que le nom et le mot de passe, puis confirme l'effacement du
# disque (« Continue with autoinstall? »). Le bureau se télécharge pendant l'installation.
#
#   image/construire-iso.sh <ubuntu-live-server.iso> <eko-rip-serveur.deb> <client.deb> <sortie.iso>
set -euo pipefail

source_iso=$1 serveur=$2 client=$3 sortie=$4
travail=$(mktemp -d)
trap 'rm -rf "$travail"' EXIT

xorriso -osirrox on -indev "$source_iso" -extract /boot/grub/grub.cfg "$travail/grub.cfg" 2>/dev/null
chmod u+w "$travail/grub.cfg"
grep -q '/casper/vmlinuz' "$travail/grub.cfg"

# Profil d'installation automatique (Subiquity). $1 : nom, $2 : bureau (oui/non),
# $3 : client (oui/non).
profil() {
    local bureau=$2 client=$3 rep="$travail/eko/profils/$1"
    mkdir -p "$rep"
    : > "$rep/meta-data"
    {
        cat <<'EOF'
#cloud-config
autoinstall:
  version: 1
  # Nom, nom du PC, compte et mot de passe demandés à l'écran ; le reste est fixé. La
  # section « identity » est absente : Subiquity refuse une identité sans mot de passe,
  # même interactive (« Malformed autoinstall in 'identity' section », vu en VM le 24/10).
  interactive-sections:
    - identity
  locale: fr_FR.UTF-8
  keyboard:
    layout: fr
  timezone: Europe/Paris
  ssh:
    install-server: true
    allow-pw: true
  storage:
    layout:
      name: direct
EOF
        if [ "$bureau" = oui ]; then
            # Bureau, et CUPS pour imprimer sur les imprimantes de bureau et du réseau.
            printf '  packages:\n    - ubuntu-desktop-minimal\n    - cups\n    - cups-browsed\n    - printer-driver-gutenprint\n    - system-config-printer\n'
        fi
        cat <<'EOF'
  late-commands:
    - cp /cdrom/eko/eko-rip-serveur.deb /cdrom/eko/eko-rip-client.deb /target/tmp/
    - curtin in-target -- sh -c 'DEBIAN_FRONTEND=noninteractive apt-get install -y /tmp/eko-rip-serveur.deb < /dev/null'
EOF
        if [ "$client" = oui ]; then
            cat <<'EOF'
    # Client installé et ouvert à chaque session ; il prend le serveur local d'office.
    - curtin in-target -- sh -c 'DEBIAN_FRONTEND=noninteractive apt-get install -y /tmp/eko-rip-client.deb < /dev/null'
    - curtin in-target -- sh -c 'mkdir -p /etc/xdg/autostart && cp /usr/share/applications/*RIP*.desktop /etc/xdg/autostart/'
EOF
        fi
        if [ "$bureau" = oui ]; then
            cat <<'EOF'
    # Suite ArtCraft (édition des fichiers) téléchargée depuis ses releases officielles,
    # empreintes vérifiées ; un échec n'arrête pas l'installation (journal, à relancer
    # plus tard par « sudo eko-artcraft »).
    - install -m 755 /cdrom/eko/eko-artcraft /target/usr/local/sbin/eko-artcraft
    - curtin in-target -- sh -c '/usr/local/sbin/eko-artcraft > /var/log/eko-artcraft.log 2>&1 || true'
    # Le bureau gère le réseau par NetworkManager (fichier copié : un « clé: valeur »
    # écrit ici serait lu comme du YAML).
    - rm -f /target/etc/netplan/50-cloud-init.yaml /target/etc/netplan/00-installer-config.yaml
    - install -m 600 /cdrom/eko/01-network-manager-all.yaml /target/etc/netplan/01-network-manager-all.yaml
EOF
        fi
        cat <<'EOF'
    - curtin in-target -- rm -f /tmp/eko-rip-serveur.deb /tmp/eko-rip-client.deb
    - mkdir -p /target/etc/issue.d
    - sh -c 'printf "E-KO RIP Center - http://\\4:8640 (client E-KO RIP Center)\n\n" > /target/etc/issue.d/eko-rip.issue'
EOF
    } > "$rep/user-data"
}

profil serveur-client oui oui
profil serveur-bureau oui non
profil serveur non non
printf 'network:\n  version: 2\n  renderer: NetworkManager\n' > "$travail/eko/01-network-manager-all.yaml"
cp "$(dirname "$0")/eko-artcraft" "$travail/eko/eko-artcraft"
cp "$serveur" "$travail/eko/eko-rip-serveur.deb"
cp "$client" "$travail/eko/eko-rip-client.deb"

# Menu GRUB (BIOS et UEFI) : nos trois entrées en tête, la première par défaut.
# EKO_OPTIONS_NOYAU : options ajoutées au noyau (essai en machine virtuelle : console série).
entree() {
    printf "menuentry '%s' {\n\tset gfxpayload=keep\n\tlinux\t/casper/vmlinuz 'ds=nocloud;s=/cdrom/eko/profils/%s/' %s---\n\tinitrd\t/casper/initrd\n}\n" "$1" "$2" "${EKO_OPTIONS_NOYAU:+$EKO_OPTIONS_NOYAU }"
}
{
    echo 'set default=0'
    echo 'set timeout=30'
    entree "E-KO RIP : serveur + client sur ce PC - EFFACE LE DISQUE" serveur-client
    entree "E-KO RIP : serveur + bureau Ubuntu - EFFACE LE DISQUE" serveur-bureau
    entree "E-KO RIP : serveur seul (sans bureau) - EFFACE LE DISQUE" serveur
    # Le menu d'origine suit, sans ses propres réglages de délai et de défaut.
    grep -v -E '^set (default|timeout)=' "$travail/grub.cfg"
} > "$travail/grub-eko.cfg"
test "$(grep -c "E-KO RIP" "$travail/grub-eko.cfg")" = 3

rm -f "$sortie"
xorriso -indev "$source_iso" -outdev "$sortie" \
    -map "$travail/grub-eko.cfg" /boot/grub/grub.cfg \
    -map "$travail/eko" /eko \
    -boot_image any replay 2>/dev/null

echo "image prête : $sortie ($(du -h "$sortie" | cut -f1))"
