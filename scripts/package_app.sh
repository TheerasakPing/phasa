#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
APP_NAME="Phasa.app"
APP_DIR="${DIST_DIR}/${APP_NAME}"
CONTENTS_DIR="${APP_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo "=== Building Phasa Release Binary ==="
cd "${ROOT_DIR}"
cargo build --release --bin phasa

echo "=== Creating Application Bundle Structure ==="
rm -rf "${APP_DIR}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

echo "=== Installing Executable ==="
cp "${ROOT_DIR}/target/release/phasa" "${MACOS_DIR}/phasa"
chmod +x "${MACOS_DIR}/phasa"

echo "=== Creating Info.plist ==="
cat << 'EOF' > "${CONTENTS_DIR}/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>phasa</string>
    <key>CFBundleIdentifier</key>
    <string>com.phasa.app</string>
    <key>CFBundleName</key>
    <string>Phasa</string>
    <key>CFBundleDisplayName</key>
    <string>Phasa (ภาษา)</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleIconFile</key>
    <string>phasa.icns</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2026 Phasa Open Source Project. All rights reserved.</string>
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

echo "=== Signing Application Bundle with Stable Designated Requirement ==="
codesign --force --deep --sign - --requirements '=designated => identifier "com.phasa.app"' "${APP_DIR}"

echo "=== Packaging Complete ==="
echo "Application bundle created at:"
echo "  ${APP_DIR}"
ls -la "${MACOS_DIR}"
ls -la "${RESOURCES_DIR}"

if [[ "${1:-}" == "--install" ]]; then
    echo "=== Installing to /Applications/Phasa.app ==="
    killall phasa 2>/dev/null || true
    rm -rf "/Applications/Phasa.app"
    cp -R "${APP_DIR}" "/Applications/Phasa.app"
    echo "Successfully installed /Applications/Phasa.app!"
    echo "=== Launching /Applications/Phasa.app ==="
    open "/Applications/Phasa.app"
fi
