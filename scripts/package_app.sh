#!/usr/bin/env bash
set -euo pipefail

# Script to build and package Phasa (ภาษา) into a standalone macOS Application Bundle (.app)
# Output: dist/Phasa.app ready for /Applications deployment

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Building Phasa Release Binary ==="
cd "${ROOT_DIR}"
cargo build --release

DIST_DIR="${ROOT_DIR}/dist"
APP_DIR="${DIST_DIR}/Phasa.app"
CONTENTS_DIR="${APP_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo "=== Creating Application Bundle Structure ==="
rm -rf "${APP_DIR}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

echo "=== Installing Executable ==="
cp "${ROOT_DIR}/target/release/phasa" "${MACOS_DIR}/phasa"
chmod +x "${MACOS_DIR}/phasa"

echo "=== Creating Info.plist ==="
cat > "${CONTENTS_DIR}/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleName</key>
    <string>Phasa</string>
    <key>CFBundleDisplayName</key>
    <string>Phasa (ภาษา)</string>
    <key>CFBundleIdentifier</key>
    <string>com.phasa.app</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleExecutable</key>
    <string>phasa</string>
    <key>CFBundleIconFile</key>
    <string>phasa</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>NSSupportsAutomaticTermination</key>
    <false/>
    <key>NSSupportsSuddenTermination</key>
    <false/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2026 Phasa Contributors.</string>
</dict>
</plist>
EOF

echo "=== Copying Application Icon ==="
PHASA_ICNS="${ROOT_DIR}/assets/phasa.icns"
if [ -f "${PHASA_ICNS}" ]; then
    cp "${PHASA_ICNS}" "${RESOURCES_DIR}/phasa.icns"
    echo "Copied Phasa icon asset (phasa.icns)."
else
    echo "Warning: phasa.icns not found at ${PHASA_ICNS}."
fi

echo "=== Copying Dictionary and Layout Assets ==="
ASSETS_DIR="${ROOT_DIR}/assets"
if [ -d "${ASSETS_DIR}" ]; then
    cp -v "${ASSETS_DIR}"/* "${RESOURCES_DIR}/"
fi

echo "=== Signing Application Bundle with Ad-Hoc Identity ==="
codesign --force --deep --sign - "${APP_DIR}"

echo "=== Packaging Complete ==="
echo "Application bundle created at:"
echo "  ${APP_DIR}"
ls -la "${MACOS_DIR}"
ls -la "${RESOURCES_DIR}"

if [[ "${1:-}" == "--install" ]]; then
    echo "=== Installing to /Applications/Phasa.app ==="
    rm -rf "/Applications/Phasa.app"
    cp -R "${APP_DIR}" "/Applications/Phasa.app"
    echo "Successfully installed /Applications/Phasa.app!"
fi
