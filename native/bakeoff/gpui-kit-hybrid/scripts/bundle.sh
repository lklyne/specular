#!/usr/bin/env bash
# Wraps the `--features cef` build in the .app layout CEF needs on macOS.
# The same layout as crates/specular-cef/scripts/bundle-macos.sh, for this
# spike's own target directory.
#
#   CEF_PATH=~/.local/share/cef CARGO_TARGET_DIR=target/cef cargo build --features cef
#   CEF_PATH=~/.local/share/cef scripts/bundle.sh
set -euo pipefail

here="$(cd "$(dirname "$0")/.." && pwd)"
name="gpui-kit-hybrid"
target_dir="$here/target/cef/debug"
bin="$target_dir/$name"
app="$target_dir/$name.app"
: "${CEF_PATH:?set CEF_PATH to the CEF binary distribution directory}"
[[ -x "$bin" ]] || { echo "missing $bin" >&2; exit 1; }

framework=""
for candidate in "$CEF_PATH" "$CEF_PATH"/*/cef_macos_aarch64 "$CEF_PATH"/*/cef_macos_x86_64; do
  if [[ -d "$candidate/Chromium Embedded Framework.framework" ]]; then
    framework="$candidate/Chromium Embedded Framework.framework"
    break
  fi
done
[[ -n "$framework" ]] || { echo "no Chromium Embedded Framework.framework under $CEF_PATH" >&2; exit 1; }

make_app() { # bundle_dir executable identifier is_helper
  mkdir -p "$1/Contents/MacOS" "$1/Contents/Resources"
  cp "$bin" "$1/Contents/MacOS/$2"
  local ui_element=""
  [[ "$4" == 1 ]] && ui_element="<key>LSUIElement</key><string>1</string>"
  cat > "$1/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>$2</string>
  <key>CFBundleIdentifier</key><string>$3</string>
  <key>CFBundleName</key><string>$2</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>NSHighResolutionCapable</key><true/>
  $ui_element
</dict></plist>
PLIST
}

rm -rf "$app"
make_app "$app" "$name" "dev.specular.gpui-kit-hybrid" 0
mkdir -p "$app/Contents/Frameworks"
cp -R "$framework" "$app/Contents/Frameworks/"
for suffix in "" " (GPU)" " (Renderer)" " (Plugin)" " (Alerts)"; do
  helper="$name Helper$suffix"
  id_suffix="$(echo "$suffix" | tr -d ' ()' | tr '[:upper:]' '[:lower:]')"
  make_app "$app/Contents/Frameworks/$helper.app" "$helper" "dev.specular.gpui-kit-hybrid.helper${id_suffix:+.$id_suffix}" 1
done
codesign --force --deep --sign - "$app" >/dev/null
echo "$app/Contents/MacOS/$name"
