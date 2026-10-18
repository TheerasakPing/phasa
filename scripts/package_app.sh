#!/usr/bin/env bash
set -euo pipefail

# Script to build and package RightLang into a standalone macOS Application Bundle (.app)
# Output: dist/RightLang.app ready for /Applications deployment

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Building RightLang Release Binary ==="
cd "${ROOT_DIR}"
cargo build --release

DIST_DIR="${ROOT_DIR}/dist"
APP_DIR="${DIST_DIR}/RightLang.app"
CONTENTS_DIR="${APP_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo "=== Creating Application Bundle Structure ==="
rm -rf "${APP_DIR}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

echo "=== Installing Executable ==="
cp "${ROOT_DIR}/target/release/rightlang-daemon" "${MACOS_DIR}/RightLang"
chmod +x "${MACOS_DIR}/RightLang"

echo "=== Creating Info.plist ==="
cat > "${CONTENTS_DIR}/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleName</key>
    <string>RightLang</string>
    <key>CFBundleDisplayName</key>
    <string>RightLang</string>
    <key>CFBundleIdentifier</key>
    <string>com.rightlang.daemon</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleExecutable</key>
    <string>RightLang</string>
    <key>CFBundleIconFile</key>
    <string>rightlang-mac</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
EOF

echo "=== Copying Application Icon ==="
ICNS_ANALYSIS="/Volumes/Data/workspace/git/REA/rightlang-analysis/extracted/RightLang.app/Contents/Resources/rightlang-mac.icns"
if [ -f "${ICNS_ANALYSIS}" ]; then
    cp "${ICNS_ANALYSIS}" "${RESOURCES_DIR}/rightlang-mac.icns"
    echo "Copied icon from analysis bundle."
else
    echo "Warning: rightlang-mac.icns not found at ${ICNS_ANALYSIS}."
fi

echo "=== Copying Dictionary and Layout Assets ==="
ASSETS_DIR="${ROOT_DIR}/assets"
if [ -d "${ASSETS_DIR}" ]; then
    cp -v "${ASSETS_DIR}"/* "${RESOURCES_DIR}/"
fi

echo "=== Packaging Complete ==="
echo "Application bundle created at:"
echo "  ${APP_DIR}"
ls -la "${MACOS_DIR}"
ls -la "${RESOURCES_DIR}"
echo ""
echo "To install to your Applications folder:"
echo "  cp -R \"${APP_DIR}\" /Applications/"
