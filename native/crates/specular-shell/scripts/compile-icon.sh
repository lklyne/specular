#!/usr/bin/env bash
# Compiles `assets/AppIcon.icon`, an Icon Composer document, into a bundle's
# Resources: `Assets.car`, which macOS 26 draws as layered glass, and a flat
# `AppIcon.icns` for the systems before it. The bundle's Info.plist names the
# icon with CFBundleIconFile and CFBundleIconName, both `AppIcon`.
#
# Usage: compile-icon.sh RESOURCES_DIR
# Needs Xcode 26 or later; exits non-zero without it, writing nothing.
set -euo pipefail

resources="$1"
src="$(cd "$(dirname "$0")/../assets" && pwd)/AppIcon.icon"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

xcrun actool "$src" --compile "$tmp" --app-icon AppIcon \
  --platform macosx --minimum-deployment-target 12.0 \
  --output-partial-info-plist "$tmp/partial.plist" >/dev/null 2>&1
[[ -f "$tmp/Assets.car" && -f "$tmp/AppIcon.icns" ]]
cp "$tmp/Assets.car" "$tmp/AppIcon.icns" "$resources/"
