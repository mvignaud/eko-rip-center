#!/bin/sh
# Monte une file CUPS « brut » d'essai pour le port IPP (roadmap 9.4) : le flux reçu est
# gardé tel quel dans /tmp/cups-sortie/brut.bin. À lancer en root sur une machine de test
# où CUPS est installé (apt install cups), puis : cargo test -- --ignored cups
set -e
mkdir -p /tmp/cups-sortie
cat > /usr/lib/cups/backend/capture <<'FIN'
#!/bin/sh
if [ $# -eq 0 ]; then echo 'direct capture "Unknown" "Capture E-KO"'; exit 0; fi
if [ -n "$6" ]; then cat "$6"; else cat; fi > /tmp/cups-sortie/brut.bin.tmp && mv /tmp/cups-sortie/brut.bin.tmp /tmp/cups-sortie/brut.bin
FIN
chmod 0700 /usr/lib/cups/backend/capture
pgrep -x cupsd >/dev/null || cupsd
sleep 1
lpadmin -p brut -v capture:/brut -E
echo "File prête : ipp://localhost:631/printers/brut"
