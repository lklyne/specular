#!/usr/bin/env bash
set -euo pipefail

# Pinned agent-browser binary. Bump VERSION + SHA256 together to update.
# Checksum from: shasum -a 256 resources/bin/agent-browser
VERSION="v0.31.1"
SHA256="fd7acd17b3071ff7f75a03c1ecd30501959d9c2d063bdaa05adb6f77abf2a7bf"

# ponytail: darwin-arm64 only — the one arch we ship. Add an arch switch when
# we make a darwin-x64 / universal build.
ASSET="agent-browser-darwin-arm64"
DEST="resources/bin/agent-browser"
URL="https://github.com/vercel-labs/agent-browser/releases/download/$VERSION/$ASSET"

# Skip the download if the pinned binary is already in place.
if [ -x "$DEST" ] && echo "$SHA256  $DEST" | shasum -a 256 -c - >/dev/null 2>&1; then
  echo "agent-browser $VERSION already present, skipping."
  exit 0
fi

echo "Fetching agent-browser $VERSION ($ASSET)..."
curl -fsSL "$URL" -o "$DEST"
echo "$SHA256  $DEST" | shasum -a 256 -c -
chmod +x "$DEST"
echo "Installed $DEST ($("$DEST" --version))"
