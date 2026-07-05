# AFK worker — per-fire instructions

You are running in a fresh cloud context. The repo is checked out on the feature branch. You have the variables `<OWNER/REPO>`, `<FEATURE_BRANCH>`, `<EPIC_ID>`, `<SELF_ROUTINE_ID>` from the kickoff prompt.

Do **one** unit of work per fire, then exit. Never wait inside a fire. If there's nothing to do, exit quickly. The cron is hourly — long fires waste tokens. The fast path between steps is `RemoteTrigger run <SELF_ROUTINE_ID>` after a state advance.

> **Implement-only fires.** When invoked by `scripts/afk-fire.sh`, a fire runs **only decision rule 4** (Start the next pending task) — an orchestrator (the launching Claude Code thread, or `scripts/afk-loop.sh`) owns rules 1–3: merging step PRs, reconciling dex, opening the integration PR. The full decision tree below still applies when a fire runs the whole tree (cloud routine, `/afk-step`).

## Branch naming

- Feature branch: `<FEATURE_BRANCH>` (already `claude/feat-<slug>`)
- Step branch you create per task: `claude/task-<short-id>` (the short dex ID, e.g. `claude/task-j4p66k15`)

Both prefixes are `claude/` so the routine's default branch permissions allow push.

## Decision tree

Walk these in order. The first matching step ends the fire (unless it says "fall through").

### 1. Is the feature already done?

```
dex show <EPIC_ID> --json
```

If every child task is `completed`:
- Make sure the feature branch has any pending dex updates committed before opening the integration PR:
  ```
  if ! git diff --quiet .dex/ || ! git diff --cached --quiet .dex/; then
    git add .dex/ && git commit -m "chore(afk): sync dex state" && git push
  fi
  ```
- Check for an integration PR: `gh pr list --head <FEATURE_BRANCH> --base main --state open --json number`
- If none exists, open it: title `<slug>: integration`, body lists every child task ID and its merged step PR.
- Disable self: `RemoteTrigger update <SELF_ROUTINE_ID> body:{"enabled": false}`.
- Exit. The user reviews and merges the integration PR.

### 2. Is a step PR open against the feature branch?

```
gh pr list --base <FEATURE_BRANCH> --state open --json number,headRefName,mergeable,mergeStateStatus
```

For each open PR targeting `<FEATURE_BRANCH>`:
- If mergeable and CI green (`mergeStateStatus` is `CLEAN`): `gh pr merge --squash --auto <num>`. Then `dex complete <task-id>` for the linked task. **Commit and push the dex state to the feature branch** so the next fire sees the completion — without this the loop's working-tree reset would wipe the change:
  ```
  git fetch origin <FEATURE_BRANCH> && git pull --ff-only
  git add .dex/ && git commit -m "chore(afk): complete <task-id>" && git push
  ```
  Then `RemoteTrigger run <SELF_ROUTINE_ID>`. Exit.
- Otherwise: exit. We're waiting for CI or human review.

### 3. Has an in-progress task's PR merged out of band?

(Covers the case where a human merged a step PR while the worker wasn't running.)

```
dex list --in-progress --json
```

For each in-progress task, find its step branch and check `gh pr list --head claude/task-<id> --state merged --limit 1`. If merged, `dex complete <task-id> --commit <sha>`. After processing all of them, **commit and push the dex state to the feature branch** so the next fire sees the completions:

```
if ! git diff --quiet .dex/; then
  git add .dex/ && git commit -m "chore(afk): sync dex state" && git push
fi
```

Then `RemoteTrigger run <SELF_ROUTINE_ID>` and exit.

### 4. Start the next pending task.

```
dex list --json
```

Filter to tasks whose `parent` is `<EPIC_ID>` and status is `pending`. Pick the first one in order whose dependencies (if any) are all `completed`. Skip tasks whose description starts with `BLOCKED:` — those are flagged for human attention. Read its full description: `dex show <task-id> --full`.

```
git switch -c claude/task-<short-id> <FEATURE_BRANCH>
```

Implement the task per its description. Stay narrowly scoped to that step — do not touch anything outside what the step describes.

Before committing, run the checks the repo expects:
- `pnpm typecheck`
- `pnpm test:unit`
- `pnpm test:integration` only if your changes touched `src/main/`, IPC, or persistence

If checks fail, see Failure handling below — do not commit broken code.

Commit, push, open PR into `<FEATURE_BRANCH>`:

```
git commit -m "<conventional message>"
git push -u origin claude/task-<short-id>
gh pr create --base <FEATURE_BRANCH> --head claude/task-<short-id> --title "<title>" --body "Closes dex task <task-id>.

<one-paragraph summary of changes>"
```

Mark the dex task in-progress:

```
dex start <task-id>
git add .dex/ && git commit --amend --no-edit && git push --force-with-lease
```

(The dex state-file update has to land on the step branch so subsequent fires see it. `--force-with-lease` is safe here because the branch is single-author and not yet merged.)

Self-trigger and exit:

```
RemoteTrigger run <SELF_ROUTINE_ID>
```

### 5. Nothing to do.

If none of the above apply — exit silently. The next cron fire (or a self-trigger from elsewhere) will look again.

## Failure handling

- **Typecheck or tests fail on a task**: do not retry blindly. Mark the task blocked and exit.
  ```
  dex edit <task-id> --description "BLOCKED: <short reason>. Original description follows:\n\n<original>"
  git add .dex/ && git commit -m "chore(afk): mark <task-id> blocked" && git push
  ```
  Decision rule 4 skips `BLOCKED:` tasks, so the worker moves on next fire.
- **Merge conflict** opening a step PR: rebase the step branch onto latest `<FEATURE_BRANCH>` once. If it doesn't rebase cleanly, mark the task `BLOCKED: merge conflict, needs human` and exit.
- **`gh` or `dex` returns an error you can't make sense of**: do not paper over. Exit the fire. Next cron tick is a fresh attempt.

## Hard rules

- Never push to `main`.
- Never `git reset --hard`, never `git push --force` (use `--force-with-lease` only on your own step branches before the PR is merged).
- Never modify the dex epic itself (only child tasks).
- Never create new RemoteTrigger routines. Only call `run` and `update enabled:false` on your own ID.
- One PR per fire, maximum. After opening or merging one, you're done — self-trigger if you advanced state, then exit.
