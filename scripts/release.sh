#!/bin/sh
# Prepare a signed, notarized and stapled ZIP. Does not publish to GitHub.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
: "$NOTARY_PROFILE"
identity=$(security find-identity -v -p codesigning | awk '/Developer ID Application:/ {print $2; exit}')
[ -n "$identity" ] || { echo "A Developer ID Application certificate is required" >&2; exit 1; }
version=0.1.0
out="$root/target/distribution/$version"
[ ! -e "$out" ] || { echo "Refusing to replace existing release output: $out" >&2; exit 1; }
mkdir -p "$out"
cd "$root"
cargo test --locked
cargo build --release --locked --target aarch64-apple-darwin
CLIP_SIGN_IDENTITY="$identity" sh scripts/package.sh target/aarch64-apple-darwin/release/clip "$out/Clipglass.app"
ditto -c -k --keepParent "$out/Clipglass.app" "$out/notarize.zip"
xcrun notarytool submit "$out/notarize.zip" --keychain-profile "$NOTARY_PROFILE" --wait --output-format json > "$out/notarization.json"
python3 - "$out/notarization.json" <<'PY'
import json,sys
result=json.load(open(sys.argv[1]))
print("Notarization:",result["status"],result["id"])
if result["status"]!="Accepted": raise SystemExit("Notarization was not accepted; do not publish")
PY
xcrun stapler staple "$out/Clipglass.app"
xcrun stapler validate "$out/Clipglass.app"
codesign --verify --deep --strict "$out/Clipglass.app"
spctl --assess --type execute --verbose=2 "$out/Clipglass.app"
ditto -c -k --keepParent "$out/Clipglass.app" "$out/Clipglass-$version-macos-arm64.zip"
cd "$out"
shasum -a 256 "Clipglass-$version-macos-arm64.zip" > SHA256SUMS.txt
echo "Verified release output: $out"
