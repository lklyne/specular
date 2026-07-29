#!/usr/bin/env bash
set -euo pipefail

# Pinned agent-browser binary. Bump VERSION + SHA256 together to update.
# Checksum from: shasum -a 256 resources/bin/agent-browser
VERSION="v0.33.1"
SHA256="33ce6a3f94322ad8ea4ac28db923737c040db88af8bb199f57778995d451f2c7"

# ponytail: darwin-arm64 only — the one arch we ship. Add an arch switch when
# we make a darwin-x64 / universal build.
ASSET="agent-browser-darwin-arm64"
DEST="resources/bin/agent-browser"
URL="https://github.com/vercel-labs/agent-browser/releases/download/$VERSION/$ASSET"

# Skip the binary download if the pinned one is already in place.
if [ -x "$DEST" ] && echo "$SHA256  $DEST" | shasum -a 256 -c - >/dev/null 2>&1; then
  echo "agent-browser $VERSION binary already present, skipping."
else
  echo "Fetching agent-browser $VERSION ($ASSET)..."
  curl -fsSL "$URL" -o "$DEST"
  echo "$SHA256  $DEST" | shasum -a 256 -c -
  chmod +x "$DEST"
  echo "Installed $DEST ($("$DEST" --version))"
fi

# The single-file binary bundles no skill content — `agent-browser skills` reads
# a `skills/` dir beside its `bin/`. We ship our own skills there; vendor
# upstream's `core` reference (snapshot refs, session mgmt, trust boundaries,
# ...) at the same pinned tag so `specular skills get core` resolves it in both
# dev (resources/skills/core) and the packaged app (Resources/skills/core).
# ponytail: tag pins the content; no separate checksum — it's reference markdown,
# not an executable. Add one if upstream ever ships a signed skills tarball.
CORE_DEST="resources/skills/core"
if [ -f "$CORE_DEST/SKILL.md" ]; then
  echo "core skill already present, skipping."
else
  echo "Fetching core skill from vercel-labs/agent-browser@$VERSION..."
  TARBALL_URL="https://github.com/vercel-labs/agent-browser/archive/refs/tags/$VERSION.tar.gz"
  TMP="$(mktemp -d)"
  trap 'rm -rf "$TMP"' EXIT
  curl -fsSL "$TARBALL_URL" | tar -xz -C "$TMP"
  SRC="$(find "$TMP" -type d -path '*/skill-data/core' -maxdepth 3 | head -1)"
  if [ -z "$SRC" ]; then
    echo "ERROR: skill-data/core not found in $VERSION tarball" >&2
    exit 1
  fi
  rm -rf "$CORE_DEST"
  mkdir -p "$(dirname "$CORE_DEST")"
  cp -R "$SRC" "$CORE_DEST"
  echo "Installed $CORE_DEST ($(find "$CORE_DEST" -type f | wc -l | tr -d ' ') files)"
fi
