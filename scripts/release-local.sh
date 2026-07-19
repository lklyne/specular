#!/usr/bin/env bash
#
# release-local.sh — build, sign, notarize, and publish a Specular release from
# your own Mac. Replaces the GitHub-hosted macOS `release.yml` job so the repo
# can stay private without burning Actions minutes (macOS runners bill at 10x).
#
# Run this AFTER the release skill has bumped the version, tagged, and pushed —
# it publishes the artifacts for the tag currently checked out.
#
# Requirements (one-time, on your Mac):
#   - Your "Developer ID Application" cert in the login keychain (already there
#     if you sign locally) — no keychain-import dance needed, unlike CI.
#   - Env vars for notarization + publish:
#       APPLE_ID           Apple ID email
#       APPLE_PASSWORD     app-specific password for that Apple ID
#       APPLE_TEAM_ID      Apple Developer team id
#       GITHUB_TOKEN       PAT with `repo` scope (electron-forge uploads to Releases)
#       SENTRY_DSN         (optional) enables Sentry symbol upload
#
# Usage:
#   export APPLE_ID=... APPLE_PASSWORD=... APPLE_TEAM_ID=... GITHUB_TOKEN=...
#   bash scripts/release-local.sh

set -euo pipefail

cd "$(dirname "$0")/.."

# --- Preflight: confirm we're on a release tag ---------------------------------
TAG="$(git describe --tags --exact-match 2>/dev/null || true)"
if [ -z "$TAG" ]; then
  echo "error: HEAD is not on a tag. Run the release skill first (bump + tag + push)," >&2
  echo "       then check out the tag before publishing." >&2
  exit 1
fi
VERSION="${TAG#v}"
echo "Publishing $TAG (version $VERSION)"

# --- Required secrets ----------------------------------------------------------
missing=0
for var in APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID GITHUB_TOKEN; do
  if [ -z "${!var:-}" ]; then
    echo "error: $var is not set" >&2
    missing=1
  fi
done
[ "$missing" -eq 0 ] || exit 1

# electron-forge's osxSign reads CSC_LINK as a signal to sign; "1" tells it to
# use an identity already present in the keychain rather than importing a .p12.
export CSC_LINK="1"

# --- Build ---------------------------------------------------------------------
pnpm install --frozen-lockfile
pnpm fetch:agent-browser
pnpm build:mcp-helper
pnpm build:cli

# --- Publish (build package, sign, notarize, upload to GitHub Releases) --------
pnpm exec electron-forge publish

# --- Populate release notes from changelog.md ----------------------------------
# Mirrors release.yml: extract this version's section and set it as the notes.
NOTES="$(mktemp)"
awk -v ver="$VERSION" '
  $0 ~ "^## \\[" ver "\\]" { found=1; next }
  found && /^## \[/ { exit }
  found {
    if ($0 ~ /^[[:space:]]*$/) { blanks = blanks "\n"; next }
    if (!started) { started = 1; blanks = "" }
    printf "%s%s\n", blanks, $0
    blanks = ""
  }
' changelog.md > "$NOTES"

if [ ! -s "$NOTES" ]; then
  echo "No changelog section for $VERSION; leaving release notes empty"
else
  for i in 1 2 3 4 5; do
    if gh release view "$TAG" >/dev/null 2>&1; then break; fi
    sleep 5
  done
  gh release edit "$TAG" --notes-file "$NOTES"
fi

echo "Done. Review the release at https://github.com/lklyne/specular/releases/tag/$TAG"
