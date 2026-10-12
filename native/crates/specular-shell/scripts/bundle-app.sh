#!/usr/bin/env bash
# Builds `Specular.app` from the GPUI Kit shell: an app someone can
# open, as opposed to the bare CEF layout `specular-cef/scripts/bundle-macos.sh`
# makes for measurement runs.
#
#   Specular.app/Contents/
#     Info.plist                                    name, id, icon, .canvas documents
#     MacOS/Specular                                the `specular` binary
#     MacOS/specular-helper                         the Agent SDK host, from native/helper
#     Helpers/claude                                the Claude Code runtime the SDK runs
#     Frameworks/Chromium Embedded Framework.framework
#     Frameworks/Specular Helper*.app               five, each holding `specular-cef-helper`
#     Resources/Assets.car, AppIcon.icns            the icon, compiled from assets/AppIcon.icon
#     Resources/starter-space/                      what a new space starts with
#
# The name, the bundle id (`com.lyleklyne.specular`) and the data folder
# (`~/Library/Application Support/Specular`) are the Electron app's, which
# this app replaces.
#
# With no --sign the signature is ad hoc. That is enough to run on the Mac
# that built it; any other Mac's Gatekeeper refuses it. A release is
# `--sign IDENTITY --notarize --dmg`: see docs/native-app-bundle.md.
#
# Usage: bundle-app.sh [--profile release|debug] [--no-build] [--out DIR]
#                      [--sign IDENTITY] [--notarize] [--dmg]
#   --profile   which build to wrap (default: release)
#   --no-build  wrap the binaries that are already built
#   --out       where to put the app (default: native/target/<profile>/bundle)
#   --sign      a Developer ID Application identity, as `security
#               find-identity` names it; signs with the hardened runtime
#   --notarize  has Apple notarize the app and staples the ticket. Needs
#               --sign, and either NOTARY_PROFILE (a `notarytool
#               store-credentials` profile) or APPLE_ID, APPLE_PASSWORD and
#               APPLE_TEAM_ID
#   --dmg       also writes Specular-<version>-<arch>.dmg beside the app,
#               notarized too under --notarize
# Needs CEF_PATH pointing at the CEF distribution; the first build downloads
# about 300 MB there. The icon needs Xcode 26 or later; without it the app
# gets the Electron app's icon.
set -euo pipefail

APP_NAME="Specular"
BUNDLE_ID="com.lyleklyne.specular"

profile="release"
build=1
out=""
identity=""
notarize=0
dmg=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile) profile="$2"; shift 2 ;;
    --no-build) build=0; shift ;;
    --out) out="$2"; shift 2 ;;
    --sign) identity="$2"; shift 2 ;;
    --notarize) notarize=1; shift ;;
    --dmg) dmg=1; shift ;;
    -h|--help) sed -n '2,38p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

[[ "$notarize" == 0 || -n "$identity" ]] || { echo "--notarize needs --sign" >&2; exit 2; }

script_dir="$(cd "$(dirname "$0")" && pwd)"
native_dir="$(cd "$script_dir/../../.." && pwd)"
repo_dir="$(cd "$native_dir/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$native_dir/target}/$profile"
out="${out:-$target_dir/bundle}"
bin="$target_dir/specular"
cef_helper_bin="$target_dir/specular-cef-helper"
app="$out/$APP_NAME.app"
: "${CEF_PATH:?set CEF_PATH to the CEF binary distribution directory}"

if [[ "$build" == 1 ]]; then
  flag=()
  [[ "$profile" == release ]] && flag=(--release)
  (cd "$native_dir" && cargo build "${flag[@]}" -p specular-shell -p specular-cef-helper --features cef)
  (cd "$native_dir/helper" && bun install --frozen-lockfile && bun run build)
fi
for built in "$bin" "$cef_helper_bin"; do
  [[ -x "$built" ]] || { echo "missing $built; build it, or drop --no-build" >&2; exit 1; }
done
helper_bin="$native_dir/helper/dist/specular-helper"
runtime_bin="$native_dir/helper/node_modules/@anthropic-ai/claude-agent-sdk-darwin-$(uname -m)/claude"
for needed in "$helper_bin" "$runtime_bin"; do
  [[ -x "$needed" ]] || { echo "missing $needed; run bun install and bun run build in native/helper" >&2; exit 1; }
done

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
  local source="$bin"
  [[ "$4" == 1 ]] && source="$cef_helper_bin"
  mkdir -p "$1/Contents/MacOS" "$1/Contents/Resources"
  cp "$source" "$1/Contents/MacOS/$2"
  # The release profile keeps line tables for profiling; a shipped app does
  # not carry them.
  strip -x "$1/Contents/MacOS/$2"
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
cp "$helper_bin" "$app/Contents/MacOS/specular-helper"
mkdir -p "$app/Contents/Helpers"
# Anthropic's own signature stays on the runtime.
cp "$runtime_bin" "$app/Contents/Helpers/claude"
mkdir -p "$app/Contents/Frameworks"
cp -R "$framework" "$app/Contents/Frameworks/"
# Every helper holds `specular-cef-helper`, which runs the child process
# Chromium's `--type=` switch names and links nothing of the app.
for suffix in "" " (GPU)" " (Renderer)" " (Plugin)" " (Alerts)"; do
  helper="$APP_NAME Helper$suffix"
  id_suffix="$(echo "$suffix" | tr -d ' ()' | tr '[:upper:]' '[:lower:]')"
  make_app "$app/Contents/Frameworks/$helper.app" "$helper" "$BUNDLE_ID.helper${id_suffix:+.$id_suffix}" 1
done

# Ad hoc with no identity. With one, the hardened runtime and a timestamp,
# which notarization requires of every executable in the bundle.
sign() { # path [entitlements]
  if [[ -z "$identity" ]]; then
    codesign --force --sign - "$1" >/dev/null 2>&1
  else
    codesign --force --sign "$identity" --options runtime --timestamp \
      ${2:+--entitlements "$2"} "$1"
  fi
}

# Chromium's renderers compile JavaScript at run time, and the agent helper
# is a Bun executable that does the same.
app_entitlements="$script_dir/entitlements/app.plist"
agent_entitlements="$script_dir/entitlements/agent-helper.plist"

# Inside out, as codesign wants nested code signed first.
cef="$app/Contents/Frameworks/Chromium Embedded Framework.framework"
while IFS= read -r library; do
  sign "$library"
done < <(find "$cef/Libraries" -type f -name '*.dylib')
sign "$cef"
for helper in "$app/Contents/Frameworks/"*.app; do
  sign "$helper" "$app_entitlements"
done
sign "$app/Contents/MacOS/specular-helper" "$agent_entitlements"
sign "$app" "$app_entitlements"
codesign --verify --deep --strict "$app"

# Sends `$1` to Apple and waits for the verdict.
submit() {
  if [[ -n "${NOTARY_PROFILE:-}" ]]; then
    xcrun notarytool submit "$1" --wait --keychain-profile "$NOTARY_PROFILE"
  else
    : "${APPLE_ID:?set NOTARY_PROFILE, or APPLE_ID, APPLE_PASSWORD and APPLE_TEAM_ID}"
    xcrun notarytool submit "$1" --wait --apple-id "$APPLE_ID" \
      --password "${APPLE_PASSWORD:?}" --team-id "${APPLE_TEAM_ID:?}"
  fi
}

if [[ "$notarize" == 1 ]]; then
  # Apple takes an archive, not a folder. The ticket is then stapled to the
  # app itself, so it opens on a Mac that is offline.
  archive="$out/notarize.zip"
  ditto -c -k --keepParent "$app" "$archive"
  submit "$archive"
  rm -f "$archive"
  xcrun stapler staple "$app"
fi
echo "$app"

if [[ "$dmg" == 1 ]]; then
  image="$out/$APP_NAME-$version-$(uname -m).dmg"
  staging="$(mktemp -d)"
  trap 'rm -rf "$staging"' EXIT
  ditto "$app" "$staging/$APP_NAME.app"
  ln -s /Applications "$staging/Applications"
  rm -f "$image"
  hdiutil create -volname "$APP_NAME" -srcfolder "$staging" -format UDZO -ov "$image" >/dev/null
  if [[ -n "$identity" ]]; then
    codesign --force --sign "$identity" --timestamp "$image"
  fi
  if [[ "$notarize" == 1 ]]; then
    submit "$image"
    xcrun stapler staple "$image"
  fi
  echo "$image"
fi
