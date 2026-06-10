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
# Two modes:
#   (default)   HEAL/IMPROVE — one fresh fire per round against the headless smoke
#               probes; fixes bugs and drains the backlog. Deterministic gate.
#   --discover  DISCOVERY — runs a workflow against the *real running app* (doer)
#               then grades the trace (independent judge), filing friction into the
#               backlog. Realistic idea-generation; it files, it does not fix.
#
# Env overrides:
#   MODEL=sonnet|opus|haiku   model for the fire (default: sonnet)
#   MAX_ROUNDS=N              stop after N rounds (default: unbounded)
#   SLEEP=N                   seconds between rounds (default: 5)
#
set -euo pipefail

# Parse args: the first non-flag is the pack; --discover selects discovery mode.
DISCOVER=0
PACK=""
for arg in "$@"; do
  case "$arg" in
    --discover) DISCOVER=1 ;;
    -*) echo "unknown flag: $arg" >&2; exit 1 ;;
    *) [[ -z "$PACK" ]] && PACK="$arg" ;;
  esac
done
PACK="${PACK:-packs/cli}"
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
if [[ "$DISCOVER" == 1 && ! -f "$PACK/workflows.md" ]]; then
  echo "discover mode needs $PACK/workflows.md" >&2
  exit 1
fi

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
  # The prompt template is domain-agnostic; the substitutions are the pack path and
  # (in discovery) the round's trace file.
  sed -e "s|__PACK__|$PACK|g" -e "s|__TRACE__|${TRACE_FILE:-}|g" "$REPO_ROOT/harness/$1"
}

if [[ "$DISCOVER" == 1 ]]; then
  # Discovery drives the real app via the canonical discovery file, so the app must
  # be running and the CLI must be built (the tracer calls out/main/cli.js).
  pnpm build:cli >/dev/null 2>&1 || { echo "discover: pnpm build:cli failed" >&2; exit 1; }
  if ! node "$REPO_ROOT/out/main/cli.js" workspace >/dev/null 2>&1; then
    echo "discover: the real app isn't reachable. Launch it first (pnpm dev)." >&2
    exit 1
  fi
  echo "discover: pack=$PACK model=$MODEL branch=$BRANCH"
else
  echo "self-heal: pack=$PACK model=$MODEL branch=$BRANCH"
fi

round=0
while true; do
  round=$((round + 1))
  if [[ "$MAX_ROUNDS" -gt 0 && "$round" -gt "$MAX_ROUNDS" ]]; then
    echo "loop: reached MAX_ROUNDS=$MAX_ROUNDS, stopping."
    break
  fi

  echo "── round $round ($(date '+%H:%M:%S')) ────────────────────────────────"

  # Refresh truth from the branch (picks up anything merged out of band).
  git pull --rebase --autostash origin "$BRANCH" 2>/dev/null || true

  if [[ "$DISCOVER" == 1 ]]; then
    # One discovery round = doer (real app, faithful trace) then independent judge.
    TRACE_FILE="$REPO_ROOT/harness/.traces/trace-$(date '+%Y%m%d-%H%M%S')-r$round.jsonl"
    mkdir -p "$(dirname "$TRACE_FILE")"
    : > "$TRACE_FILE"; : > "$TRACE_FILE.meta"
    export SPECULAR_TRACE_FILE="$TRACE_FILE"

    echo "discover: doer (trace: $TRACE_FILE)"
    claude -p "$(render_prompt discover-doer.md)" \
      --model "$MODEL" \
      --allowedTools "Bash,Read,Glob,Grep" \
      --dangerously-skip-permissions || echo "discover: doer exited non-zero (round $round)"

    echo "discover: judge"
    claude -p "$(render_prompt discover-judge.md)" \
      --model "$MODEL" \
      --allowedTools "Bash,Read,Write,Edit,Glob,Grep" \
      --dangerously-skip-permissions || echo "discover: judge exited non-zero (round $round)"
  else
    # One fresh-context fire. It reads the pack, does one unit of work, runs the
    # probes, and commits + records the outcome in the pack's backlog.md itself.
    claude -p "$(render_prompt fire.md)" \
      --model "$MODEL" \
      --allowedTools "Bash,Read,Write,Edit,Glob,Grep" \
      --dangerously-skip-permissions || echo "self-heal: fire exited non-zero (round $round)"
  fi

  # Push whatever the round committed (no-op if it committed nothing).
  git push -u origin "$BRANCH" 2>/dev/null || echo "loop: nothing to push (round $round)"

  sleep "$SLEEP"
done
