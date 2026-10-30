#!/bin/bash
# Construit « E-KO RIP Center.app » (universelle arm64 + x86_64, signée ad hoc) puis le
# .dmg « E-KO RIP Center_<version>_universal.dmg ». À lancer sur un Mac (Xcode ou outils en
# ligne de commande). Sortie dans clients/mac/dist/.
#
# La version est lue dans le Cargo.toml racine ([workspace.package] version).
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
APP="$SORTIE/$NOM.app"

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

# 5. Signature ad hoc (pas de compte développeur : au premier lancement, clic droit > Ouvrir).
codesign --force --deep -s - "$APP"
codesign --verify --deep --strict "$APP"

# 6. Image disque.
DMG="$SORTIE/${NOM}_${VERSION}_universal.dmg"
PREPARATION="$(mktemp -d)"
cp -R "$APP" "$PREPARATION/"
ln -s /Applications "$PREPARATION/Applications"
rm -f "$DMG"
hdiutil create -volname "$NOM" -srcfolder "$PREPARATION" -ov -format UDZO "$DMG"
rm -rf "$PREPARATION"

echo "==> $APP"
echo "==> $DMG"
