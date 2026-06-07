#!/usr/bin/env bash
#
# Self-improvement engine — general purpose.
#
# Knows nothing about the CLI (or any domain). It points a fresh-context agent at
# a "pack" of docs and loops. The pack decides what "better" means, what to probe,
# and what the guardrails are. To self-heal the CLI:   ./harness/loop.sh packs/cli
#
# Each iteration is a fresh `claude -p` (no -c): no context accumulates in the
# loop, so cost per round is flat. Durable state lives in git — the pack's
# backlog.md (the memory) and the commits the fire makes.
#
# Env overrides:
#   MODEL=sonnet|opus|haiku   model for the fire (default: sonnet)
#   MAX_ROUNDS=N              stop after N rounds (default: unbounded)
#   SLEEP=N                   seconds between rounds (default: 5)
#
set -euo pipefail

PACK="${1:-packs/cli}"
MODEL="${MODEL:-sonnet}"
MAX_ROUNDS="${MAX_ROUNDS:-0}"
SLEEP="${SLEEP:-5}"

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

if [[ ! -d "$PACK" ]]; then
  echo "pack not found: $PACK" >&2
  exit 1
fi
for doc in charter.md guardrails.md backlog.md probes.md; do
  if [[ ! -f "$PACK/$doc" ]]; then
    echo "pack is missing $PACK/$doc" >&2
    exit 1
  fi
done

# Safety rail (the dumb version of CI gating): never run the loop on a primary
# branch. The fire commits straight to whatever branch is checked out; that
# branch is meant to be reviewed before it reaches main.
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
case "$BRANCH" in
  main|master)
    echo "refuse to run the self-heal loop on '$BRANCH'. Check out a self-heal branch first." >&2
    exit 1
    ;;
esac

render_prompt() {
  # The prompt template is domain-agnostic; the only substitution is the pack path.
  sed "s|__PACK__|$PACK|g" "$REPO_ROOT/harness/fire.md"
}

echo "self-heal: pack=$PACK model=$MODEL branch=$BRANCH"

round=0
while true; do
  round=$((round + 1))
  if [[ "$MAX_ROUNDS" -gt 0 && "$round" -gt "$MAX_ROUNDS" ]]; then
    echo "self-heal: reached MAX_ROUNDS=$MAX_ROUNDS, stopping."
    break
  fi

  echo "── round $round ($(date '+%H:%M:%S')) ────────────────────────────────"

  # Refresh truth from the branch (picks up anything merged out of band).
  git pull --rebase --autostash origin "$BRANCH" 2>/dev/null || true

  # One fresh-context fire. It reads the pack, does one unit of work, runs the
  # probes, and commits + records the outcome in the pack's backlog.md itself.
  claude -p "$(render_prompt)" \
    --model "$MODEL" \
    --allowedTools "Bash,Read,Write,Edit,Glob,Grep" \
    --dangerously-skip-permissions || echo "self-heal: fire exited non-zero (round $round)"

  # Push whatever the fire committed (no-op if it committed nothing).
  git push -u origin "$BRANCH" 2>/dev/null || echo "self-heal: nothing to push (round $round)"

  sleep "$SLEEP"
done
