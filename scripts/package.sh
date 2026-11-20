#!/bin/sh
# Package an already-built development binary. This never launches or replaces an app.
set -eu
binary=${1:?usage: package.sh BINARY NEW_APP_PATH}
app=${2:?usage: package.sh BINARY NEW_APP_PATH}
if [ -e "$app" ]; then
    echo "Refusing to replace existing bundle: $app" >&2
    exit 1
fi
mkdir -p "$app/Contents/MacOS"
cp "$binary" "$app/Contents/MacOS/clip"
chmod 755 "$app/Contents/MacOS/clip"
python3 - "$app/Contents/Info.plist" <<'PY'
import plistlib
import sys
with open(sys.argv[1], "wb") as out:
    plistlib.dump({
        "CFBundleExecutable": "clip",
        "CFBundleIdentifier": "com.moritz.clip",
        "CFBundleName": "Clip",
        "CFBundleDisplayName": "Clip",
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": "0.1.0",
        "CFBundleVersion": "1",
        "LSMinimumSystemVersion": "26.0",
        "LSUIElement": True,
        "NSHighResolutionCapable": True,
    }, out)
PY
codesign --force --sign - "$app"
codesign --verify --strict "$app"
echo "$app"
