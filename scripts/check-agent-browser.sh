#!/usr/bin/env bash
set -euo pipefail

# Compare the pinned agent-browser version against the latest upstream release.
# Informational only — bumping is a human call. Prints release notes + a compare
# link when behind so you can judge whether it's worth it.
REPO="vercel-labs/agent-browser"
PINNED=$(grep -oE 'VERSION="v[0-9.]+"' scripts/fetch-agent-browser.sh | grep -oE 'v[0-9.]+')
LATEST=$(gh release view --repo "$REPO" --json tagName --jq .tagName)

echo "pinned: $PINNED"
echo "latest: $LATEST"

if [ "$PINNED" = "$LATEST" ]; then
  echo "up to date."
  exit 0
fi

echo
echo "behind — release notes from $PINNED → $LATEST:"
echo "  https://github.com/$REPO/compare/$PINNED...$LATEST"
echo
gh release view "$LATEST" --repo "$REPO" --json name,body --jq '.name + "\n\n" + .body' | head -60
echo
echo "to bump: set VERSION=\"$LATEST\" in scripts/fetch-agent-browser.sh, clear SHA256,"
echo "then run pnpm fetch:agent-browser (it prints the new hash to paste back)."
