#!/usr/bin/env bash
# Compiles `assets/AppIcon.icon`, an Icon Composer document, into a bundle's
# Resources: `Assets.car`, which macOS 26 draws as layered glass, and a flat
# `AppIcon.icns` for the systems before it. The bundle's Info.plist names the
# icon with CFBundleIconFile and CFBundleIconName, both `AppIcon`.
#
# Usage: compile-icon.sh RESOURCES_DIR
# Needs an Xcode that reads the document, which the one on CI does not;
# without it the compiled copy in `assets/AppIcon.compiled` is used.
# `UPDATE_COMPILED_ICON=1` refreshes that copy after the icon changes.
set -euo pipefail

resources="$1"
src="$(cd "$(dirname "$0")/../assets" && pwd)/AppIcon.icon"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

xcrun actool "$src" --compile "$tmp" --app-icon AppIcon \
  --platform macosx --minimum-deployment-target 12.0 \
  --output-partial-info-plist "$tmp/partial.plist" >"$tmp/actool.log" 2>&1 || true
if [[ ! -f "$tmp/Assets.car" || ! -f "$tmp/AppIcon.icns" ]]; then
  # An actool older than the document's format cannot open it, which is the
  # case on CI. The copy compiled on a Mac that can is used instead.
  echo "actool could not compile the icon; using assets/AppIcon.compiled" >&2
  cp "$src/../AppIcon.compiled/Assets.car" "$src/../AppIcon.compiled/AppIcon.icns" "$resources/"
  exit 0
fi
cp "$tmp/Assets.car" "$tmp/AppIcon.icns" "$resources/"
# Kept in the repo for a build whose actool cannot compile the document.
# Commit it when the icon changes.
if [[ "${UPDATE_COMPILED_ICON:-0}" == 1 ]]; then
  cp "$tmp/Assets.car" "$tmp/AppIcon.icns" "$src/../AppIcon.compiled/"
fi
