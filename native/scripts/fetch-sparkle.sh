#!/usr/bin/env bash
# Downloads the Sparkle distribution the app's updater comes from, checks it
# against the pinned hash and prints the folder it is in. Does nothing when
# it is already there.
#
# The folder holds `Sparkle.framework`, which the bundle embeds, and
# `bin/sign_update`, which signs an update archive for the feed.
#
# Usage: fetch-sparkle.sh
#   SPARKLE_PATH  where to keep it (default ~/.local/share/sparkle/<version>)
set -euo pipefail

VERSION="2.10.0"
SHA256="c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c"

dir="${SPARKLE_PATH:-$HOME/.local/share/sparkle/$VERSION}"
if [[ ! -d "$dir/Sparkle.framework" ]]; then
  archive="$(mktemp -d)/Sparkle-$VERSION.tar.xz"
  curl -fsSL -o "$archive" \
    "https://github.com/sparkle-project/Sparkle/releases/download/$VERSION/Sparkle-$VERSION.tar.xz"
  echo "$SHA256  $archive" | shasum -a 256 -c - >/dev/null \
    || { echo "Sparkle $VERSION does not match its pinned hash" >&2; exit 1; }
  mkdir -p "$dir"
  tar -xf "$archive" -C "$dir"
  rm -f "$archive"
fi
echo "$dir"
