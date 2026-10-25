#!/usr/bin/env bash
# AFK feature loop — unattended orchestrator.
#
# Drives a multi-PR feature build to completion without a human in the loop.
# Each round it does ONE orchestration step:
#   - all child tasks done    -> open the integration PR into main, stop
#   - a step PR is open       -> wait for its CI, merge it, advance dex
#   - tasks remain            -> run scripts/afk-fire.sh (one implement-only
#                                worker fire) to open the next step PR
#
# The expensive code work happens in stateless `claude -p` fires spawned by
# afk-fire.sh — fresh context each time, no compaction. The cheap mechanical
# work (watching CI, merging, advancing dex) happens here in the shell, so it
# never burns a model fire.
#
# The attended equivalent — the Claude Code thread that ran /afk-local acting
# as the orchestrator — is specified in .claude/skills/afk-feature/orchestrator.md.
# Use this script for fully unattended runs, or as crash-recovery for an
# attended run (all state lives in git + dex + open PRs).
#
# Usage:
#   ./scripts/afk-loop.sh                 # auto-detect epic from dex
#   ./scripts/afk-loop.sh <epic-id>       # explicit epic ID
#
# Env knobs:
#   MAX_ROUNDS        max orchestration rounds before stopping (default 40)
#   SLEEP_SECONDS     seconds to sleep between rounds (default 5)
#   CI_POLL_SECONDS   seconds between CI re-checks while a PR has no checks yet
#                     (default 15)
#   AFK_WORKER        which CLI runs each fire: "claude" (default) or "codex"
#   AFK_SOFT_CHECKS   comma-separated CI check names allowed to fail without
#                     blocking a merge (default: none)
#
# Stop with Ctrl+C.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

FEATURE_BRANCH="$(git branch --show-current)"
if [[ ! "$FEATURE_BRANCH" =~ ^claude/feat- ]]; then
  echo "Error: not on a claude/feat-* branch (currently: $FEATURE_BRANCH)" >&2
  echo "       Switch to your feature branch and try again." >&2
  exit 1
fi

EPIC_ID="${1:-}"
if [[ -z "$EPIC_ID" ]]; then
  EPIC_ID="$(dex list --json | python3 -c '
import sys, json
data = json.load(sys.stdin)
for t in data:
    if t.get("parent_id") is None and t.get("children") and not t.get("completed"):
        print(t["id"])
        break
')"
fi

if [[ -z "$EPIC_ID" ]]; then
  echo "Error: could not auto-detect a pending epic from dex." >&2
  echo "       Pass it as arg 1: ./scripts/afk-loop.sh <epic-id>" >&2
  exit 1
fi

OWNER_REPO="$(gh repo view --json nameWithOwner -q .nameWithOwner)"
MAX_ROUNDS="${MAX_ROUNDS:-40}"
SLEEP_SECONDS="${SLEEP_SECONDS:-5}"
CI_POLL_SECONDS="${CI_POLL_SECONDS:-15}"
AFK_WORKER="${AFK_WORKER:-claude}"
AFK_SOFT_CHECKS="${AFK_SOFT_CHECKS:-}"
export AFK_WORKER AFK_SOFT_CHECKS

case "$AFK_WORKER" in
  claude|codex) ;;
  *)
    echo "Error: AFK_WORKER must be 'claude' or 'codex' (got: $AFK_WORKER)" >&2
    exit 1
    ;;
esac

if ! command -v "$AFK_WORKER" >/dev/null 2>&1; then
  echo "Error: '$AFK_WORKER' CLI is not on PATH." >&2
  exit 1
fi

echo "=== AFK loop (unattended orchestrator) ==="
echo "Repo:           $OWNER_REPO"
echo "Feature branch: $FEATURE_BRANCH"
echo "Epic:           $EPIC_ID"
echo "Worker:         $AFK_WORKER"
echo "Max rounds:     $MAX_ROUNDS"
echo "Soft checks:    $AFK_SOFT_CHECKS"
echo ""

# --- helpers --------------------------------------------------------------

# Echo the epic's state: "done" | "children-done" | "in-progress".
epic_state() {
  dex show "$EPIC_ID" --json 2>/dev/null | python3 -c '
import sys, json
try:
    t = json.load(sys.stdin)
    if t.get("completed"):
        print("done")
    else:
        children = t.get("subtasks", {}).get("children", []) or t.get("children", [])
        if children and all(c.get("completed") for c in children):
            print("children-done")
        else:
            print("in-progress")
except Exception:
    print("in-progress")
'
}

# Echo the number of the first open step PR targeting the feature branch, if any.
open_step_pr() {
  gh pr list --base "$FEATURE_BRANCH" --state open --json number \
    --jq '.[0].number // empty'
}

# Block until the given PR's CI checks finish, then echo "pass" or
# "fail:<names>". Checks named in AFK_SOFT_CHECKS may fail without blocking.
wait_for_ci() {
  local pr="$1"
  local attempt rc
  for attempt in 1 2 3 4 5 6; do
    # `gh pr checks --watch` blocks until every check completes.
    # Exit 8 = no checks registered yet (PR just opened) -> wait and retry.
    if gh pr checks "$pr" --watch >/dev/null 2>&1; then
      break
    fi
    rc=$?
    if [[ "$rc" -eq 8 ]]; then
      sleep "$CI_POLL_SECONDS"
      continue
    fi
    # rc 1: checks finished, at least one failed -> fall through to evaluate.
    break
  done
  gh pr view "$pr" --json statusCheckRollup --jq '.statusCheckRollup' \
    | python3 -c '
import sys, json, os
soft = set(s for s in os.environ.get("AFK_SOFT_CHECKS", "").split(",") if s)
rollup = json.load(sys.stdin) or []
failed = []
for c in rollup:
    name = c.get("name") or c.get("context") or "?"
    concl = (c.get("conclusion") or "").upper()
    state = (c.get("state") or "").upper()
    if concl:
        ok = concl in ("SUCCESS", "NEUTRAL", "SKIPPED")
    elif state:
        ok = state == "SUCCESS"
    else:
        ok = True
    if not ok and name not in soft:
        failed.append(name)
print("fail:" + ",".join(failed) if failed else "pass")
'
}

# Open the integration PR for the feature branch into main.
open_integration_pr() {
  local slug body
  slug="${FEATURE_BRANCH#claude/feat-}"
  body="$(dex show "$EPIC_ID" --json 2>/dev/null | python3 -c '
import sys, json
t = json.load(sys.stdin)
children = t.get("subtasks", {}).get("children", []) or t.get("children", [])
print("Integration PR for the AFK feature build.\n")
print("Completed tasks:")
for c in children:
    print("- `%s` %s" % (c.get("id", "?"), c.get("name", "")))
print("\nEach task shipped as a step PR into the feature branch. Review the")
print("squashed history on this branch before merging to main.")
')"
  gh pr create \
    --base main --head "$FEATURE_BRANCH" \
    --title "${slug}: integration" \
    --body "$body"
}

# --- orchestration loop ---------------------------------------------------

prev_action=""

for round in $(seq 1 "$MAX_ROUNDS"); do
  echo "=== Round $round/$MAX_ROUNDS — $(date -u +%H:%M:%SZ) ==="

  # Refresh the working tree to the remote feature branch.
  git switch "$FEATURE_BRANCH" >/dev/null 2>&1 || true
  git fetch origin "$FEATURE_BRANCH" >/dev/null 2>&1 || true
  git reset --hard "origin/$FEATURE_BRANCH" >/dev/null 2>&1 || true

  state="$(epic_state)"

  # Step 1: feature finished?
  if [[ "$state" == "done" ]]; then
    echo "✓ Epic $EPIC_ID is complete. Loop done."
    exit 0
  fi

  if [[ "$state" == "children-done" ]]; then
    integ="$(gh pr list --head "$FEATURE_BRANCH" --base main --state open \
      --json number --jq '.[0].number // empty')"
    if [[ -n "$integ" ]]; then
      echo "✓ All tasks done; integration PR #$integ already open against main."
    else
      echo "All tasks done — opening the integration PR..."
      open_integration_pr
    fi
    echo "Loop done. Review and merge the integration PR."
    exit 0
  fi

  # Step 2: a step PR is open — wait for CI, then merge it.
  pr="$(open_step_pr)"
  if [[ -n "$pr" ]]; then
    echo "Step PR #$pr is open. Waiting for CI..."
    verdict="$(wait_for_ci "$pr")"
    if [[ "$verdict" == "pass" ]]; then
      head="$(gh pr view "$pr" --json headRefName --jq .headRefName)"
      task_id="${head#claude/task-}"
      echo "CI green — merging PR #$pr (task $task_id)..."
      gh pr merge "$pr" --squash --delete-branch
      git fetch origin "$FEATURE_BRANCH" >/dev/null 2>&1 || true
      git switch "$FEATURE_BRANCH" >/dev/null 2>&1 || true
      git reset --hard "origin/$FEATURE_BRANCH" >/dev/null 2>&1 || true
      dex complete "$task_id" \
        --result "Merged step PR #$pr into $FEATURE_BRANCH." \
        --commit "$(git rev-parse HEAD)"
      git add .dex/
      git commit -m "chore(afk): complete $task_id"
      git push
      prev_action="merge"
    else
      echo "✗ CI failed on PR #$pr (${verdict#fail:})." >&2
      echo "  Stopping for human review — fix the PR, then re-run this loop." >&2
      exit 1
    fi
    echo "Sleeping ${SLEEP_SECONDS}s before next round..."
    sleep "$SLEEP_SECONDS"
    continue
  fi

  # No open step PR. If the previous round already ran a fire and still no PR
  # appeared, the worker opened nothing — remaining tasks are likely BLOCKED.
  if [[ "$prev_action" == "implement" ]]; then
    echo "✗ Last fire opened no step PR — remaining tasks are likely BLOCKED" >&2
    echo "  or the worker failed. Stopping for human review." >&2
    exit 1
  fi

  # Step 4: implement the next pending task in a fresh worker fire.
  echo "No open step PR — running an implement-only worker fire..."
  if ! "$SCRIPT_DIR/afk-fire.sh" "$EPIC_ID"; then
    echo "afk-fire.sh exited non-zero, stopping" >&2
    exit 1
  fi
  prev_action="implement"

  echo ""
  echo "Sleeping ${SLEEP_SECONDS}s before next round..."
  sleep "$SLEEP_SECONDS"
done

echo "Reached MAX_ROUNDS=$MAX_ROUNDS without completing the epic. Stopping."
