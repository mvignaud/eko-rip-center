#!/bin/bash
# Construit « E-KO RIP Center.app » (universelle arm64 + x86_64) puis le
# .dmg « E-KO RIP Center_<version>_universal.dmg ». À lancer sur un Mac (Xcode ou outils en
# ligne de commande). Sortie dans clients/mac/dist/.
#
# Signature : avec le certificat « Developer ID Application » du trousseau (ou celui nommé par
# EKO_SIGNATURE), sinon ad hoc. Notarisation si le profil notarytool « eko-notarisation »
# existe ; à créer une fois à la main (mot de passe d'application de appleid.apple.com) :
#   xcrun notarytool store-credentials eko-notarisation --apple-id <identifiant> --team-id <TEAMID>
#
# La version est lue dans le Cargo.toml racine ([workspace.package] version).
#
# Avec les seuls outils en ligne de commande 27 (Xcode absent ou non sélectionné), le SDK
# macOS 27 ne fournit pas le greffon des macros SwiftUI (@State) : compiler avec le SDK 26,
#   SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk ./construire.sh
# et lancer les tests avec
#   swift test -Xswiftc -plugin-path -Xswiftc /Library/Developer/CommandLineTools/usr/lib/swift/host/plugins/testing
# Avec Xcode sélectionné (xcode-select -s, licence acceptée), rien de cela n'est nécessaire.
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "construire.sh : à lancer sur un Mac." >&2
    exit 1
fi

ICI="$(cd "$(dirname "$0")" && pwd)"
RACINE="$(cd "$ICI/../.." && pwd)"
ICONES="$RACINE/clients/icone"
SORTIE="$ICI/dist"
NOM="E-KO RIP Center"
# Assemblage et signature hors du dossier source : sous ~/Documents (iCloud Drive), File
# Provider repose aussitôt com.apple.FinderInfo sur le paquet et codesign le refuse.
ATELIER="$(mktemp -d)"
trap 'rm -rf "$ATELIER"' EXIT
APP="$ATELIER/$NOM.app"

# Version du dépôt : première ligne « version = "x.y.z" » de la section [workspace.package].
VERSION="$(awk '
    /^\[workspace\.package\]/ { dedans = 1; next }
    /^\[/                     { dedans = 0 }
    dedans && /^version[[:space:]]*=/ { gsub(/[" ]/, "", $3); print $3; exit }
' "$RACINE/Cargo.toml")"
if [[ -z "$VERSION" ]]; then
    echo "construire.sh : version introuvable dans $RACINE/Cargo.toml" >&2
    exit 1
fi
echo "==> E-KO RIP Center $VERSION"

# 1. Compilation universelle.
cd "$ICI"
swift build -c release --arch arm64 --arch x86_64
BIN="$(swift build -c release --arch arm64 --arch x86_64 --show-bin-path)/EKORIPCenter"
echo "==> Architectures : $(lipo -archs "$BIN")"

# 2. Arborescence de l'application.
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/fr.lproj"
cp "$BIN" "$APP/Contents/MacOS/EKORIPCenter"
# Le dossier fr.lproj fait parler le système (menus, panneaux) en français.
: > "$APP/Contents/Resources/fr.lproj/InfoPlist.strings"
printf 'APPL????' > "$APP/Contents/PkgInfo"

# 3. Info.plist.
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>$NOM</string>
    <key>CFBundleDisplayName</key>
    <string>$NOM</string>
    <key>CFBundleIdentifier</key>
    <string>fr.e-ko.rip-center</string>
    <key>CFBundleExecutable</key>
    <string>EKORIPCenter</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleDevelopmentRegion</key>
    <string>fr</string>
    <key>CFBundleLocalizations</key>
    <array>
        <string>fr</string>
    </array>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.graphics-design</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>NSHumanReadableCopyright</key>
    <string>E-KO</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Fichier d'impression grand format</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSHandlerRank</key>
            <string>Alternate</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>com.adobe.pdf</string>
                <string>public.svg-image</string>
                <string>public.tiff</string>
                <string>public.png</string>
                <string>public.jpeg</string>
            </array>
        </dict>
    </array>
    <key>NSAppTransportSecurity</key>
    <dict>
        <!-- Le serveur RIP parle HTTP sur le réseau local. -->
        <key>NSAllowsArbitraryLoads</key>
        <true/>
        <key>NSAllowsLocalNetworking</key>
        <true/>
    </dict>
</dict>
</plist>
PLIST
plutil -lint "$APP/Contents/Info.plist"

# 4. Icône : AppIcon.icns à partir de clients/icone/icone-*.png.
ICONSET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET"
cp "$ICONES/icone-16.png"   "$ICONSET/icon_16x16.png"
cp "$ICONES/icone-32.png"   "$ICONSET/icon_16x16@2x.png"
cp "$ICONES/icone-32.png"   "$ICONSET/icon_32x32.png"
cp "$ICONES/icone-64.png"   "$ICONSET/icon_32x32@2x.png"
cp "$ICONES/icone-128.png"  "$ICONSET/icon_128x128.png"
cp "$ICONES/icone-256.png"  "$ICONSET/icon_128x128@2x.png"
cp "$ICONES/icone-256.png"  "$ICONSET/icon_256x256.png"
cp "$ICONES/icone-512.png"  "$ICONSET/icon_256x256@2x.png"
cp "$ICONES/icone-512.png"  "$ICONSET/icon_512x512.png"
cp "$ICONES/icone-1024.png" "$ICONSET/icon_512x512@2x.png"
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"
rm -rf "$(dirname "$ICONSET")"

# 5. Signature : Developer ID si disponible (EKO_SIGNATURE, ou le seul certificat
#    « Developer ID Application » du trousseau), ad hoc sinon.
SIGNATURE="${EKO_SIGNATURE:-}"
# codesign refuse les attributs étendus (Finder, provenance) que le dossier source dépose.
xattr -cr "$APP"
if [[ -z "$SIGNATURE" ]]; then
    IDENTITES="$(security find-identity -v -p codesigning | sed -n 's/.*"\(Developer ID Application: .*\)"$/\1/p')"
    if [[ -n "$IDENTITES" && "$(printf '%s\n' "$IDENTITES" | wc -l)" -eq 1 ]]; then
        SIGNATURE="$IDENTITES"
    fi
fi
if [[ -n "$SIGNATURE" ]]; then
    echo "==> Signature : $SIGNATURE"
    DROITS="$(mktemp)"
    # Pas de bac à sable : seul l'accès réseau sortant est déclaré.
    cat > "$DROITS" <<DROITS
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.network.client</key>
    <true/>
</dict>
</plist>
DROITS
    codesign --force --options runtime --timestamp --entitlements "$DROITS" -s "$SIGNATURE" "$APP"
    rm -f "$DROITS"
else
    echo "==> Signature AD HOC : aucun certificat Developer ID (au premier lancement, clic droit > Ouvrir)."
    codesign --force --deep -s - "$APP"
fi
codesign --verify --deep --strict "$APP"

# 6. Image disque.
DMG="$ATELIER/${NOM}_${VERSION}_universal.dmg"
PREPARATION="$ATELIER/volume"
mkdir "$PREPARATION"
cp -R "$APP" "$PREPARATION/"
ln -s /Applications "$PREPARATION/Applications"
hdiutil create -volname "$NOM" -srcfolder "$PREPARATION" -ov -format UDZO "$DMG"

# 7. Signature et notarisation du .dmg (Developer ID seulement ; notarisation si le profil
#    notarytool « eko-notarisation » existe).
if [[ -n "$SIGNATURE" ]]; then
    codesign --force --timestamp -s "$SIGNATURE" "$DMG"
    if xcrun notarytool history --keychain-profile eko-notarisation >/dev/null 2>&1; then
        echo "==> Notarisation…"
        xcrun notarytool submit "$DMG" --keychain-profile eko-notarisation --wait
        xcrun stapler staple "$DMG"
        spctl -a -vv -t open --context context:primary-signature "$DMG"
    else
        echo "==> Pas de notarisation : profil notarytool « eko-notarisation » absent."
    fi
fi

# 8. Copie dans dist/ (l'app copiée y reprend des attributs étendus : c'est le .dmg qui fait foi).
mkdir -p "$SORTIE"
rm -rf "$SORTIE/$NOM.app"
ditto "$APP" "$SORTIE/$NOM.app"
cp "$DMG" "$SORTIE/"
echo "==> $SORTIE/$NOM.app"
echo "==> $SORTIE/$(basename "$DMG")"
