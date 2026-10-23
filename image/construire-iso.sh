#!/usr/bin/env bash
# Construit l'image d'installation du serveur E-KO RIP Center à partir de l'ISO
# netinst officielle de Debian. L'ISO d'origine est modifiée par xorriso, qui rejoue
# son démarrage BIOS et UEFI tel quel : seuls les menus changent, et le dossier /eko
# (préconfiguration + paquet serveur) s'ajoute.
#
#   image/construire-iso.sh <debian-netinst.iso> <eko-rip-serveur.deb> <sortie.iso>
set -euo pipefail

source_iso=$1 deb=$2 sortie=$3
ici=$(cd "$(dirname "$0")" && pwd)
travail=$(mktemp -d)
trap 'rm -rf "$travail"' EXIT

extraire() { xorriso -osirrox on -indev "$source_iso" -extract "$1" "$travail/$2" 2>/dev/null; chmod u+w "$travail/$2"; }
extraire /isolinux/menu.cfg menu.cfg
extraire /isolinux/gtk.cfg gtk.cfg
extraire /boot/grub/grub.cfg grub.cfg
extraire /md5sum.txt md5sum.txt

options="auto=true priority=critical preseed/file=/cdrom/eko/preseed.cfg locale=fr_FR.UTF-8 keymap=fr hostname=eko-rip domain=local"
libelle="Installer E-KO RIP - EFFACE LE DISQUE"
remplacer() { awk "$1" "$2" > "$travail/tmp" && mv "$travail/tmp" "$2"; }

# BIOS (isolinux) : notre entrée d'abord, et par défaut.
cat > "$travail/eko.cfg" <<EOF
label eko
	menu label ^$libelle
	menu default
	kernel /install.amd/vmlinuz
	append vga=788 initrd=/install.amd/initrd.gz $options --- quiet
EOF
remplacer '/^include gtk.cfg$/ { print "include eko.cfg" } { print }' "$travail/menu.cfg"
remplacer '/^default installgui$/ { print "default eko"; next } /^\tmenu default$/ { next } { print }' "$travail/gtk.cfg"
grep -q '^include eko.cfg$' "$travail/menu.cfg"
grep -q '^default eko$' "$travail/gtk.cfg"

# UEFI (grub) : notre entrée avant « Graphical install ».
cat > "$travail/entree.cfg" <<EOF
menuentry '$libelle' {
    set background_color=black
    linux    /install.amd/vmlinuz vga=788 $options --- quiet
    initrd   /install.amd/initrd.gz
}
EOF
awk 'FNR == NR { entree = entree $0 "\n"; next }
     /^menuentry .*Graphical install/ && !fait { printf "%s", entree; fait = 1 } { print }' \
    "$travail/entree.cfg" "$travail/grub.cfg" > "$travail/tmp" && mv "$travail/tmp" "$travail/grub.cfg"
grep -q "E-KO RIP" "$travail/grub.cfg"

mkdir "$travail/eko"
cp "$ici/preseed.cfg" "$travail/eko/preseed.cfg"
cp "$deb" "$travail/eko/eko-rip-serveur.deb"

# Somme de contrôle de « Vérifier l'intégrité du support » : lignes à jour pour les
# fichiers changés ou ajoutés (isolinux/ n'y figure pas dans l'ISO Debian).
somme() { if command -v md5sum >/dev/null; then md5sum "$1" | cut -d' ' -f1; else md5 -q "$1"; fi; }
remplacer '$2 != "./boot/grub/grub.cfg" { print }' "$travail/md5sum.txt"
for f in boot/grub/grub.cfg:grub.cfg eko/preseed.cfg:eko/preseed.cfg eko/eko-rip-serveur.deb:eko/eko-rip-serveur.deb; do
    echo "$(somme "$travail/${f#*:}")  ./${f%%:*}" >> "$travail/md5sum.txt"
done

rm -f "$sortie"
xorriso -indev "$source_iso" -outdev "$sortie" \
    -map "$travail/eko.cfg" /isolinux/eko.cfg \
    -map "$travail/menu.cfg" /isolinux/menu.cfg \
    -map "$travail/gtk.cfg" /isolinux/gtk.cfg \
    -map "$travail/grub.cfg" /boot/grub/grub.cfg \
    -map "$travail/md5sum.txt" /md5sum.txt \
    -map "$travail/eko" /eko \
    -boot_image any replay 2>/dev/null

echo "image prête : $sortie ($(du -h "$sortie" | cut -f1))"
