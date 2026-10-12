#!/usr/bin/env bash
# Builds `Specular.app` from the GPUI Kit shell: an app someone can
# open, as opposed to the bare CEF layout `specular-cef/scripts/bundle-macos.sh`
# makes for measurement runs.
#
#   Specular.app/Contents/
#     Info.plist                                    name, id, icon, .canvas documents
#     MacOS/Specular                                the `specular` binary
#     MacOS/specular-helper                         the Agent SDK host and the CLI, from native/helper
#     Helpers/claude                                the Claude Code runtime the SDK runs
#     Frameworks/Chromium Embedded Framework.framework
#     Frameworks/Specular Helper*.app               five, each holding `specular-cef-helper`
#     Frameworks/Sparkle.framework                  the updater (scripts/fetch-sparkle.sh)
#     Resources/Assets.car, AppIcon.icns            the icon, compiled from assets/AppIcon.icon
#     Resources/starter-space/                      what a new space starts with
#     Resources/specular-cli.sh                     the `specular` command a link on the PATH points at
#     Resources/bin/agent-browser                   the CLI's browse verbs (scripts/build-agent-browser.sh)
#     Resources/skills/                             the specular skill, and the ones agent-browser reads
#
# The name, the bundle id (`com.lyleklyne.specular`) and the data folder
# (`~/Library/Application Support/Specular`) are the Electron app's, which
# this app replaces.
#
# With no --sign the signature is ad hoc. That is enough to run on the Mac
# that built it; any other Mac's Gatekeeper refuses it. A release is
# `--sign IDENTITY --notarize --dist`: see docs/native-app-bundle.md.
#
# Usage: bundle-app.sh [--profile release|debug] [--no-build] [--out DIR]
#                      [--sign IDENTITY] [--notarize] [--dist]
#   --profile   which build to wrap (default: release)
#   --no-build  wrap the binaries that are already built
#   --out       where to put the app (default: native/target/<profile>/bundle)
#   --sign      a Developer ID Application identity, as `security
#               find-identity` names it; signs with the hardened runtime
#   --notarize  has Apple notarize the app and staples the ticket. Needs
#               --sign, and either NOTARY_PROFILE (a `notarytool
#               store-credentials` profile) or APPLE_ID, APPLE_PASSWORD and
#               APPLE_TEAM_ID
#   --dist      also writes, beside the app, what a release publishes:
#               Specular-<version>-<arch>.dmg to download (notarized too
#               under --notarize), Specular-<version>-<arch>.zip for the
#               updater, and appcast.xml, the feed that names the zip. The
#               feed is signed with SPARKLE_PRIVATE_KEY, else with the key
#               in the login keychain under the account `specular`
# Needs CEF_PATH pointing at the CEF distribution; the first build downloads
# about 300 MB there.
set -euo pipefail

APP_NAME="Specular"
BUNDLE_ID="com.lyleklyne.specular"
# Where a release's files are downloaded from. `latest` follows the newest
# release, so the feed's address never changes.
RELEASES="https://github.com/lklyne/specular/releases"
FEED_URL="$RELEASES/latest/download/appcast.xml"
# The public half of the key updates are signed with. The private half is
# in the maintainer's keychain and the release workflow's secrets.
UPDATE_PUBLIC_KEY="fNjQ/Sg+trxPWPfXND/2sBcjgdq61Eve9QqKRXxlvQA="

profile="release"
build=1
out=""
identity=""
notarize=0
dist=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile) profile="$2"; shift 2 ;;
    --no-build) build=0; shift ;;
    --out) out="$2"; shift 2 ;;
    --sign) identity="$2"; shift 2 ;;
    --notarize) notarize=1; shift ;;
    --dist) dist=1; shift ;;
    -h|--help) sed -n '2,44p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
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
# Built whatever --no-build says: its own cache makes a second run quick.
agent_browser="${AGENT_BROWSER_BIN:-$("$native_dir/scripts/build-agent-browser.sh" | tail -n 1)}"
[[ -x "$agent_browser" ]] || { echo "missing agent-browser at $agent_browser" >&2; exit 1; }
# The source tree the binary was built in holds the skill it reads.
core_skill="${AGENT_BROWSER_CORE_SKILL:-${agent_browser%/cli/target/release/agent-browser}/skill-data/core}"
[[ -f "$core_skill/SKILL.md" ]] || { echo "missing agent-browser's core skill at $core_skill" >&2; exit 1; }
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
sparkle="$("$native_dir/scripts/fetch-sparkle.sh")"

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
  <key>SUFeedURL</key><string>$FEED_URL</string>
  <key>SUPublicEDKey</key><string>$UPDATE_PUBLIC_KEY</string>
  <key>SUEnableAutomaticChecks</key><true/>
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
"$script_dir/compile-icon.sh" "$app/Contents/Resources"
make_app "$app" "$APP_NAME" "$BUNDLE_ID" 0
cp -R "$repo_dir/resources/starter-space" "$app/Contents/Resources/starter-space"
cp "$helper_bin" "$app/Contents/MacOS/specular-helper"
cp "$script_dir/specular-cli.sh" "$app/Contents/Resources/specular-cli.sh"
mkdir -p "$app/Contents/Resources/bin" "$app/Contents/Resources/skills"
cp "$agent_browser" "$app/Contents/Resources/bin/agent-browser"
cp -R "$repo_dir/resources/skills/specular" "$app/Contents/Resources/skills/specular"
cp -R "$core_skill" "$app/Contents/Resources/skills/core"
mkdir -p "$app/Contents/Helpers"
# Anthropic's own signature stays on the runtime.
cp "$runtime_bin" "$app/Contents/Helpers/claude"
mkdir -p "$app/Contents/Frameworks"
cp -R "$framework" "$app/Contents/Frameworks/"
cp -R "$sparkle/Sparkle.framework" "$app/Contents/Frameworks/"
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
# Sparkle's own parts, in the order its documentation gives. The downloader
# keeps the entitlements it shipped with.
updater="$app/Contents/Frameworks/Sparkle.framework"
sign "$updater/Versions/B/XPCServices/Installer.xpc"
if [[ -z "$identity" ]]; then
  sign "$updater/Versions/B/XPCServices/Downloader.xpc"
else
  codesign --force --sign "$identity" --options runtime --timestamp \
    --preserve-metadata=entitlements "$updater/Versions/B/XPCServices/Downloader.xpc"
fi
sign "$updater/Versions/B/Autoupdate"
sign "$updater/Versions/B/Updater.app"
sign "$updater"
for helper in "$app/Contents/Frameworks/"*.app; do
  sign "$helper" "$app_entitlements"
done
sign "$app/Contents/MacOS/specular-helper" "$agent_entitlements"
sign "$app/Contents/Resources/bin/agent-browser"
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

if [[ "$dist" == 1 ]]; then
  name="$APP_NAME-$version-$(uname -m)"
  image="$out/$name.dmg"
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

  # The updater downloads the app as a zip, which holds the stapled app.
  update="$out/$name.zip"
  rm -f "$update"
  ditto -c -k --sequesterRsrc --keepParent "$app" "$update"
  echo "$update"

  # `sign_update` prints the two attributes the feed's enclosure carries:
  # the archive's signature and its length.
  if [[ -n "${SPARKLE_PRIVATE_KEY:-}" ]]; then
    signature="$(printf '%s' "$SPARKLE_PRIVATE_KEY" | "$sparkle/bin/sign_update" --ed-key-file - "$update")"
  else
    signature="$("$sparkle/bin/sign_update" --account specular "$update")"
  fi
  cat > "$out/appcast.xml" <<FEED
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>$APP_NAME</title>
    <item>
      <title>$version</title>
      <pubDate>$(date -u '+%a, %d %b %Y %H:%M:%S +0000')</pubDate>
      <sparkle:version>$version</sparkle:version>
      <sparkle:shortVersionString>$version</sparkle:shortVersionString>
      <sparkle:minimumSystemVersion>12.0</sparkle:minimumSystemVersion>
      <sparkle:releaseNotesLink>$RELEASES/tag/v$version</sparkle:releaseNotesLink>
      <enclosure url="$RELEASES/download/v$version/$name.zip" type="application/octet-stream" $signature/>
    </item>
  </channel>
</rss>
FEED
  xmllint --noout "$out/appcast.xml"
  echo "$out/appcast.xml"
fi
