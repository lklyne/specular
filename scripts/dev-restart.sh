#!/usr/bin/env bash
# Cleanly kill any running Specular dev process and restart.
# Can be run from Claude's sandbox — launches via osascript into a real Terminal window.
#
# Usage:
#   pnpm dev:restart          — kill + restart in a new Terminal tab
#   pnpm dev:restart:debug    — same, but with custom CDP port (default 9333 is always on)
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CDP_PORT=9333
APP_CONTROL_PORT=29979

# ── Kill existing Specular processes ──────────────────────────────
echo "Stopping Specular..."

# Kill the entire process tree: Electron, forge, AND child processes (Vite dev server).
# SIGKILL doesn't propagate to children on macOS, so kill Vite/node children explicitly.
pkill -9 -f "Electron.app.*Specular" 2>/dev/null || true
pkill -9 -f "electron-forge start" 2>/dev/null || true

# Kill orphaned Vite dev servers and node processes from the project directory.
# These survive pkill -9 of the parent forge process.
pgrep -f "node.*vite.*${PROJECT_DIR}" | xargs kill -9 2>/dev/null || true
pgrep -f "node.*electron-forge.*${PROJECT_DIR}" | xargs kill -9 2>/dev/null || true

# Wait for CDP port to close
for i in {1..10}; do
  curl -s "http://127.0.0.1:${CDP_PORT}/json/version" >/dev/null 2>&1 || break
  sleep 0.5
done

# ── Relaunch in a real Terminal window ─────────────────────────────
echo "Launching pnpm dev in Terminal..."
DEV_CMD="cd ${PROJECT_DIR} && pnpm dev"
if [[ -n "${CDP:-}" ]]; then
  DEV_CMD="cd ${PROJECT_DIR} && SPECULAR_REMOTE_DEBUGGING_PORT=${CDP_PORT} pnpm dev"
fi

osascript -e "tell application \"Terminal\" to do script \"${DEV_CMD}\""

# ── Wait for the app to be fully ready ────────────────────────────
# CDP opens early (before renderers load). Wait for the app-control server
# health endpoint instead — it starts after initWindow() + renderer loading.
echo "Waiting for Specular app-control server on port ${APP_CONTROL_PORT}..."
for i in {1..60}; do
  if curl -s "http://127.0.0.1:${APP_CONTROL_PORT}/health" 2>/dev/null | grep -q '"version"'; then
    echo "Specular ready (${i}s)"
    exit 0
  fi
  sleep 1
done

echo "Warning: app-control server not ready after 60s — app may still be starting"
exit 1
