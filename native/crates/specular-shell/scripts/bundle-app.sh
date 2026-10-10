#!/usr/bin/env bash
# Builds `Specular Native.app` from the GPUI Kit shell: an app someone can
# open, as opposed to the bare CEF layout `specular-cef/scripts/bundle-macos.sh`
# makes for measurement runs.
#
#   Specular Native.app/Contents/
#     Info.plist                                    name, id, icon, .canvas documents
#     MacOS/Specular Native                         the `specular` binary
#     Frameworks/Chromium Embedded Framework.framework
#     Frameworks/Specular Native Helper*.app        five copies of the binary
#     Resources/Assets.car, AppIcon.icns            the icon, compiled from assets/AppIcon.icon
#     Resources/starter-space/                      what a new space starts with
#
# The name and the bundle id differ from the Electron app's (`Specular`,
# `com.lyleklyne.specular`), so both install side by side. Nothing on disk
# is shared: this app's data folder is
# `~/Library/Application Support/Specular Native`.
#
# The signature is ad hoc. That is enough to run on the Mac that built it;
# any other Mac's Gatekeeper refuses it. Developer ID signing, notarization,
# a disk image and updates are not done here: see docs/native-app-bundle.md.
#
# Usage: bundle-app.sh [--profile release|debug] [--no-build] [--out DIR]
#   --profile   which build to wrap (default: release)
#   --no-build  wrap the binary that is already built
#   --out       where to put the app (default: native/target/<profile>/bundle)
# Needs CEF_PATH pointing at the CEF distribution; the first build downloads
# about 300 MB there. The icon needs Xcode 26 or later; without it the app
# gets the Electron app's icon.
set -euo pipefail

APP_NAME="Specular Native"
BUNDLE_ID="com.lyleklyne.specular.native"

profile="release"
build=1
out=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile) profile="$2"; shift 2 ;;
    --no-build) build=0; shift ;;
    --out) out="$2"; shift 2 ;;
    -h|--help) sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

native_dir="$(cd "$(dirname "$0")/../../.." && pwd)"
repo_dir="$(cd "$native_dir/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$native_dir/target}/$profile"
out="${out:-$target_dir/bundle}"
bin="$target_dir/specular"
app="$out/$APP_NAME.app"
: "${CEF_PATH:?set CEF_PATH to the CEF binary distribution directory}"

if [[ "$build" == 1 ]]; then
  flag=()
  [[ "$profile" == release ]] && flag=(--release)
  (cd "$native_dir" && cargo build "${flag[@]}" -p specular-shell --features cef)
fi
[[ -x "$bin" ]] || { echo "missing $bin; build it, or drop --no-build" >&2; exit 1; }

framework=""
for candidate in "$CEF_PATH" "$CEF_PATH"/*/cef_macos_aarch64 "$CEF_PATH"/*/cef_macos_x86_64; do
  if [[ -d "$candidate/Chromium Embedded Framework.framework" ]]; then
    framework="$candidate/Chromium Embedded Framework.framework"
    break
  fi
done
[[ -n "$framework" ]] || { echo "no Chromium Embedded Framework.framework under $CEF_PATH" >&2; exit 1; }

version="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "$native_dir/Cargo.toml")"
: "${version:?no version in native/Cargo.toml}"

# `.canvas` is JSON Canvas, which no system type covers, so the type is
# declared here as an imported one. The rank is Alternate: the app opens a
# canvas from Finder without taking the extension from an app that owns it.
# No URL scheme: the Electron app registers none with the system, and a
# scheme both apps claimed would go to whichever the system saw last.
main_keys() {
  cat <<PLIST
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleIconFile</key><string>$icon_name</string>
  <key>CFBundleIconName</key><string>$icon_name</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>CFBundleDocumentTypes</key>
  <array><dict>
    <key>CFBundleTypeName</key><string>Canvas</string>
    <key>CFBundleTypeRole</key><string>Editor</string>
    <key>LSHandlerRank</key><string>Alternate</string>
    <key>LSItemContentTypes</key><array><string>org.jsoncanvas.canvas</string></array>
  </dict></array>
  <key>UTImportedTypeDeclarations</key>
  <array><dict>
    <key>UTTypeIdentifier</key><string>org.jsoncanvas.canvas</string>
    <key>UTTypeDescription</key><string>JSON Canvas</string>
    <key>UTTypeConformsTo</key><array><string>public.json</string></array>
    <key>UTTypeTagSpecification</key>
    <dict><key>public.filename-extension</key><array><string>canvas</string></array></dict>
  </dict></array>
PLIST
}

write_plist() { # path executable identifier is_helper
  local extra
  if [[ "$4" == 1 ]]; then
    extra="<key>LSUIElement</key><string>1</string>"
  else
    extra="$(main_keys)"
  fi
  cat > "$1" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>$2</string>
  <key>CFBundleIdentifier</key><string>$3</string>
  <key>CFBundleName</key><string>$2</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>LSEnvironment</key><dict><key>MallocNanoZone</key><string>0</string></dict>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  $extra
</dict></plist>
PLIST
  plutil -lint -s "$1"
}

make_app() { # bundle_dir executable identifier is_helper
  mkdir -p "$1/Contents/MacOS" "$1/Contents/Resources"
  cp "$bin" "$1/Contents/MacOS/$2"
  write_plist "$1/Contents/Info.plist" "$2" "$3" "$4"
}

rm -rf "$app"
mkdir -p "$app/Contents/Resources"
icon_name="AppIcon"
if ! "$(dirname "$0")/compile-icon.sh" "$app/Contents/Resources"; then
  echo "actool could not compile the icon; using the Electron app's" >&2
  icon_name="icon"
  cp "$repo_dir/build/icon.icns" "$app/Contents/Resources/icon.icns"
fi
make_app "$app" "$APP_NAME" "$BUNDLE_ID" 0
cp -R "$repo_dir/resources/starter-space" "$app/Contents/Resources/starter-space"
mkdir -p "$app/Contents/Frameworks"
cp -R "$framework" "$app/Contents/Frameworks/"
# Every helper is the same binary: Chromium's `--type=` switch makes it a
# child process (`run_subprocess_if_needed`).
for suffix in "" " (GPU)" " (Renderer)" " (Plugin)" " (Alerts)"; do
  helper="$APP_NAME Helper$suffix"
  id_suffix="$(echo "$suffix" | tr -d ' ()' | tr '[:upper:]' '[:lower:]')"
  make_app "$app/Contents/Frameworks/$helper.app" "$helper" "$BUNDLE_ID.helper${id_suffix:+.$id_suffix}" 1
done

# Inside out, as codesign wants nested code signed first.
codesign --force --sign - "$app/Contents/Frameworks/Chromium Embedded Framework.framework" >/dev/null 2>&1
for helper in "$app/Contents/Frameworks/"*.app; do
  codesign --force --sign - "$helper" >/dev/null 2>&1
done
codesign --force --sign - "$app" >/dev/null 2>&1
codesign --verify --deep --strict "$app"
echo "$app"
