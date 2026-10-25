---
name: afk-step
description: Run one fire of the AFK feature worker decision tree locally. Use when you want to manually advance a multi-PR feature build one step at a time on your laptop — pick up the next pending task, implement it, open a step PR, merge a mergeable PR, or advance dex state. One unit of work per invocation, then exits. The local equivalent of the cloud worker routine.
---

# AFK step

One fire of the AFK feature worker, run locally in the current Claude Code session.

Use this when you've kicked off an AFK feature via `/afk-feature` (or already have a `claude/feat-*` branch + dex epic set up) and want to advance it manually one step at a time. Re-invoke for each subsequent step. To self-pace, wrap in `/loop /afk-step`.

## When to use this skill vs `scripts/afk-loop.sh`

| | This skill | `scripts/afk-loop.sh` |
|---|---|---|
| Context | Reuses the current Claude Code session — prompt cache stays warm, context grows across fires | Fresh `claude -p` per fire — no cache reuse, no compaction risk |
| Cost | Lower per-fire while session is small; spikes when harness compacts | Fixed per-fire; predictable |
| Control | One step per invocation; you re-trigger | Loops automatically until epic done or max fires |
| Recommended for | Quick test of one step; debugging a stuck task | Long unattended runs of 5+ steps |

## What this skill does

1. **Verify state.** Confirm we're checked out on a `claude/feat-*` branch. If not, abort and tell the user to switch.
2. **Resolve the epic ID.**
    - If the user passed it as an argument, use that.
    - Otherwise auto-detect: `dex list --json` and pick the first top-level task (`parent_id: null`) that has `children` populated. If multiple epics exist, prompt the user to pick.
3. **Set variables for worker.md.**
    - `<OWNER/REPO>` ← `gh repo view --json nameWithOwner -q .nameWithOwner`
    - `<FEATURE_BRANCH>` ← current branch
    - `<EPIC_ID>` ← from step 2
    - `<SELF_ROUTINE_ID>` ← N/A locally; skip every `RemoteTrigger.run`/`RemoteTrigger.update` instruction in worker.md
4. **Read `.claude/skills/afk-feature/worker.md`** and execute its decision tree **once**. Stop after the first matching rule fires.
5. **Exit summary.** Print one of:
    - "Started task <id>: <name>. Opened PR #<n>."
    - "Merged PR #<n> for task <id>. Task complete; next pending: <id> <name>."
    - "Opened integration PR #<n>. Epic complete."
    - "Nothing to do — all step PRs waiting for CI or human review (PR #<n>: <status>)."

## Local-mode behavior differences

Worker.md was written for the cloud routine. When running locally, skip:

- `RemoteTrigger run <SELF_ROUTINE_ID>` — no self-trigger; the user (or the loop script) handles re-firing.
- `RemoteTrigger update <SELF_ROUTINE_ID> {"enabled": false}` — no self-disable; user disables manually if needed.

Everything else (git, gh, dex, cargo checks) runs identically.

## Hard rules (inherited from worker.md)

- Never push to `main`.
- Never `git reset --hard`, never `git push --force` (use `--force-with-lease` only on your own step branches before the PR is merged).
- One PR per fire, maximum.
- Skip tasks whose description starts with `BLOCKED:`.

## Invocation

```
/afk-step
/afk-step <epic-id>     # if auto-detect picks the wrong epic
```
