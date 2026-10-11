#!/usr/bin/env bash
set -euo pipefail

# Builds agent-browser from source with the Specular pre-act hook
# (native/patches/agent-browser-will-act.patch). The hook sends the CDP command
# `Specular.willAct` before each targeted action so the canvas can animate the
# agent cursor to the target first. Bump VERSION and regenerate the patch together.
# Last line of stdout is the absolute path of the built binary.
VERSION="v0.38.1"
REPO_URL="https://github.com/vercel-labs/agent-browser"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PATCH="$ROOT/native/patches/agent-browser-will-act.patch"
SRC="${AGENT_BROWSER_SRC:-$HOME/.cache/specular/agent-browser-src-$VERSION}"

if [ ! -d "$SRC/.git" ]; then
  echo "Cloning agent-browser $VERSION into $SRC..." >&2
  mkdir -p "$(dirname "$SRC")"
  git clone --quiet --depth 1 --branch "$VERSION" "$REPO_URL" "$SRC" >&2
else
  echo "Resetting $SRC to $VERSION..." >&2
  git -C "$SRC" fetch --quiet --depth 1 origin "refs/tags/$VERSION:refs/tags/$VERSION" >&2
fi

# Back to the pristine tag so the patch always applies, however many times this runs.
# Ignored paths (cli/target, build.rs's placeholder dashboard dir) survive the clean.
git -C "$SRC" reset --quiet --hard "$VERSION"
git -C "$SRC" clean --quiet -fd

echo "Applying $(basename "$PATCH")..." >&2
git -C "$SRC" apply "$PATCH"

# The dashboard bundle that rust-embed embeds is not in the repo; build.rs writes a
# placeholder packages/dashboard/out/index.html when it is missing. That is enough
# here: we only drive the CLI, never the dashboard UI.
# Uses the clone's own toolchain file if it has one (cwd = cli/ picks it up).
echo "Building (cargo build --release)..." >&2
(cd "$SRC/cli" && cargo build --release >&2)

BIN="$SRC/cli/target/release/agent-browser"
"$BIN" --version >&2
echo "$BIN"
