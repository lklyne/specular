#!/usr/bin/env bash
# Wraps a built `specular-app` binary in the .app layout CEF requires on macOS:
#
#   specular-app.app/Contents/
#     MacOS/specular-app                         browser process
#     Resources/Assets.car, AppIcon.icns         the icon, when Xcode can compile it
#     Frameworks/Chromium Embedded Framework.framework
#     Frameworks/specular-app Helper.app          utility/network children
#     Frameworks/specular-app Helper (GPU).app
#     Frameworks/specular-app Helper (Renderer).app
#     Frameworks/specular-app Helper (Plugin).app
#     Frameworks/specular-app Helper (Alerts).app
#
# Every helper is a copy of the same binary: `run_subprocess_if_needed` sees
# Chromium's `--type=` switch and runs the child. Mirrors
# cef::build_util::mac::bundle, which this script exists to avoid needing a
# separate bundler crate and a debug-only `cargo build`.
#
# Usage: bundle-macos.sh [profile] [binary]   (default: release specular-app)
# `binary` is `specular` for the GPUI Kit shell (`-p specular-shell`).
# Needs CEF_PATH pointing at the CEF distribution the build used (set it for
# the build too, so the download is reused).
set -euo pipefail

profile="${1:-release}"
native_dir="$(cd "$(dirname "$0")/../../.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$native_dir/target}/$profile"
name="${2:-specular-app}"
bin="$target_dir/$name"
app="$target_dir/$name.app"

[[ -x "$bin" ]] || { echo "missing $bin; build it first with --features cef" >&2; exit 1; }
: "${CEF_PATH:?set CEF_PATH to the CEF binary distribution directory}"

framework=""
for candidate in "$CEF_PATH" "$CEF_PATH"/*/cef_macos_aarch64 "$CEF_PATH"/*/cef_macos_x86_64; do
  if [[ -d "$candidate/Chromium Embedded Framework.framework" ]]; then
    framework="$candidate/Chromium Embedded Framework.framework"
    break
  fi
done
[[ -n "$framework" ]] || { echo "no Chromium Embedded Framework.framework under $CEF_PATH" >&2; exit 1; }

write_plist() { # path executable identifier is_helper
  local extra="$icon_keys"
  [[ "$4" == 1 ]] && extra="<key>LSUIElement</key><string>1</string>"
  cat > "$1" <<PLIST
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
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>LSEnvironment</key><dict><key>MallocNanoZone</key><string>0</string></dict>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  $extra
</dict></plist>
PLIST
}

make_app() { # bundle_dir executable identifier is_helper
  mkdir -p "$1/Contents/MacOS" "$1/Contents/Resources"
  cp "$bin" "$1/Contents/MacOS/$2"
  write_plist "$1/Contents/Info.plist" "$2" "$3" "$4"
}

rm -rf "$app"
mkdir -p "$app/Contents/Resources"
icon_keys=""
if "$native_dir/crates/specular-shell/scripts/compile-icon.sh" "$app/Contents/Resources"; then
  icon_keys="<key>CFBundleIconFile</key><string>AppIcon</string><key>CFBundleIconName</key><string>AppIcon</string>"
fi
make_app "$app" "$name" "dev.specular.spike" 0
mkdir -p "$app/Contents/Frameworks"
cp -R "$framework" "$app/Contents/Frameworks/"
for suffix in "" " (GPU)" " (Renderer)" " (Plugin)" " (Alerts)"; do
  helper="$name Helper$suffix"
  id_suffix="$(echo "$suffix" | tr -d ' ()' | tr '[:upper:]' '[:lower:]')"
  make_app "$app/Contents/Frameworks/$helper.app" "$helper" "dev.specular.spike.helper${id_suffix:+.$id_suffix}" 1
done

# Ad-hoc signature: enough for local runs on Apple Silicon (unsigned arm64
# code is killed), not for distribution.
codesign --force --deep --sign - "$app" >/dev/null
echo "$app"
