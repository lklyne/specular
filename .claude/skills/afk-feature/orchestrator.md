# AFK orchestrator — per-thread instructions

You are the **orchestrator** for an AFK feature build, running in the Claude Code
thread that launched `/afk-local`. You drive a multi-PR feature to completion by
spawning stateless worker fires for the code work and handling the cheap
mechanical steps (CI, merges, dex) yourself.

You have the variables `<OWNER/REPO>`, `<FEATURE_BRANCH>` (`claude/feat-<slug>`),
and `<EPIC_ID>` from the kickoff.

`scripts/afk-loop.sh` is the unattended bash equivalent of this loop. This file
is the attended version — same decision tree, run by you.

## Prime directive: stay lean

You will run for the whole feature, so your context must not bloat.

- **Never read or edit feature-code files.** That is the worker's job.
- **Never read a PR diff.** You merge based on CI status, not code inspection.
- Spawn each worker as a separate process (`scripts/afk-fire.sh`) — its context
  (reading code, holding diffs) lives in *its* process. You ingest only its short
  stdout summary.
- Run CI waits and worker fires as **background** commands and let the completion
  notification wake you. Do not poll, do not `sleep`-loop.
- Keep your own turns short: a status line per round, nothing more.

## The loop

Repeat until the feature is done. Each round, first `git fetch origin` and read
the epic: `dex show <EPIC_ID> --json`.

### 1. Feature finished?

If every child task is `completed`:
- If an integration PR is already open (`gh pr list --head <FEATURE_BRANCH>
  --base main --state open`), you are done.
- Otherwise open it: base `main`, head `<FEATURE_BRANCH>`, title
  `<slug>: integration`. Write a **real** body — list each Part, its merged step
  PR number, and the test results. This is the one place you summarize; it is
  worth doing well because the user reviews this PR.
- Stop. Tell the user the integration PR is open for review. **Never merge it
  yourself** — that is the user's call.

### 2. A step PR is open against the feature branch?

`gh pr list --base <FEATURE_BRANCH> --state open --json number,headRefName`

If one exists, wait for its CI without spending a fire — run this as a
**background** command and continue when notified:

```
gh pr checks <pr> --watch
```

When it finishes, read the rollup (`gh pr view <pr> --json statusCheckRollup`):
- If every check passed **except** ones on the soft-allowlist (`AFK_SOFT_CHECKS`,
  empty by default) →
  merge: `gh pr merge <pr> --squash --delete-branch`. Then advance dex: derive
  the task id from the branch name (`claude/task-<id>` → `<id>`), run
  `dex complete <id> --result "Merged step PR #<pr>." --commit <squash-sha>`,
  then commit and push `.dex/` to the feature branch.
- If a non-soft check failed → **stop**. Surface the failing check to the user;
  do not merge, do not start the next task.

### 3. Tasks remain — spawn an implement-only worker fire

Run as a **background** command; continue when notified it exited:

```
./scripts/afk-fire.sh <EPIC_ID>
```

This spawns one fresh headless `claude -p` that implements the next pending dex
task and opens a step PR into the feature branch — nothing else. Read only its
final summary. Next round, step 2 will pick up the PR it opened.

If a fire exits and **no** new step PR appeared (and the feature is not
finished), the remaining tasks are likely `BLOCKED` or the worker failed — stop
and ask the user.

## Recovery

All state lives in git, dex, and open PRs — not in your context. If this thread
dies, a new orchestrator (or `scripts/afk-loop.sh`) resumes by reading the same
epic and open PRs. Nothing is lost.

## Hard rules

- Never push to `main`. Never merge the integration PR.
- Never `git reset --hard` outside the loop's own working-tree refresh.
- One worker fire in flight at a time — merge the open step PR before spawning
  the next fire, so each task branches off the prior task's merged work.
