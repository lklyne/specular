---
name: afk-local
description: Kick off an AFK multi-PR feature build locally using scripts/afk-loop.sh. Accepts either a planning doc under docs/plans/ OR a GitHub issue (number or URL). Sets up the feature branch + dex epic + subtasks, then drops into the local loop. Use when the user wants to "run this AFK locally", "loop on this issue", or "AFK from issue #N". Sibling of /afk-feature (which sets up the cloud routine instead).
---

# AFK local

One-shot kickoff for a multi-PR feature built by the **local** worker loop (`scripts/afk-loop.sh`).

Input: a path to a planning doc under `docs/plans/` **or** a GitHub issue reference (`#81`, `81`, or full URL).

Output: a `claude/feat-<slug>` branch with the plan committed, a dex epic with one subtask per worker-codeable step, and an orchestration loop running the build.

## Architectural notes

- The build runs as an **orchestrator + stateless worker fires**. `scripts/afk-fire.sh` is one implement-only worker fire (a fresh `claude -p` — no compaction, predictable cost). The orchestrator handles the cheap mechanical steps (watch CI, merge step PRs, advance dex, open the integration PR) so they never burn a model fire.
- Two orchestrators exist: the launching Claude Code thread (attended — see `.claude/skills/afk-feature/orchestrator.md`) or `scripts/afk-loop.sh` (unattended bash). Step 10 lets the user pick.
- Subtasks come from `## Phase N` (or `## <heading>`) sections in the plan. The user picks which ones are worker-codeable before any dex tasks are created — "Decisions before starting" / "Goals" / "Non-goals" / "Success criteria" style sections usually shouldn't be tasks.
- Each fire reads dex state from the working tree, so `.dex/` must be committed to the feature branch.
- Both scripts enforce `claude/feat-*` branch naming — they exit if the branch doesn't match.

## Kickoff sequence

### 1. Resolve the input

If the user passed:
- A path ending in `.md` (or matching `docs/plans/*`): treat as a **plan file**. Read it; you'll derive the slug from its filename.
- A bare number, `#N`, or a URL containing `/issues/N`: treat as an **issue ref**. Fetch with `gh issue view <N> --json title,body,url`.
- Nothing: stop and ask the user to provide one.

### 2. Derive the slug

- **Plan file:** slug = filename without `.md`. (e.g. `docs/plans/keyboard-binding-registry.md` → `keyboard-binding-registry`.)
- **Issue:** propose a tidy slug from the first comma/colon-delimited segment of the title, kebab-cased, capped at ~30 chars. Show the user and let them override.

Feature branch: `claude/feat-<slug>`. Plan path (issue mode only): `docs/plans/<slug>.md`.

### 3. Identify worker-codeable steps

Parse the plan/issue body. Find `## ` headings. Show the user a numbered list of every `## ` section and your proposed **worker-codeable** subset (typically the `## Phase N` rows, minus any "decisions / goals / non-goals / success criteria / risks" sections).

Ask the user to confirm or edit which sections become dex subtasks.

### 4. Confirm the plan before writing anything

Show the user:

```
Slug:     <slug>
Branch:   claude/feat-<slug>
Plan:     docs/plans/<slug>.md   (created)   OR   <existing path>
Subtasks: (N)
  - <section name>
  - <section name>
  ...
```

Wait for explicit confirmation. Don't proceed silently.

### 5. Create the feature branch

```
git fetch origin nightly
git switch -c claude/feat-<slug> origin/nightly
```

If the branch already exists locally or on origin, switch to it instead of recreating.

### 6. Write the plan file (issue mode only)

If the plan file does not exist, write the issue body to `docs/plans/<slug>.md` with a header containing the title and source URL. Commit:

```
git add docs/plans/<slug>.md
git commit -m "chore(afk): ingest issue #<N> plan"
```

### 7. Ingest the plan into dex

```
dex plan docs/plans/<slug>.md
```

Capture the new root task ID — the simplest way is to snapshot `dex list --json` filtered to root tasks (`parent_id is None`) before and after the call, and take the new ID from the diff.

### 8. Create subtasks

For each confirmed section from step 3, in order:

```
dex create "<section heading>" --parent <epic-id> --description "<section body>"
```

(If a child task with that name already exists under the epic, skip — keeps the skill idempotent if the user re-runs it.)

### 9. Commit `.dex/` and push

```
git add .dex/
git commit -m "chore(afk): seed dex subtasks for <slug>"
git push -u origin claude/feat-<slug>
```

### 10. Hand off to orchestration

Tell the user the kickoff is complete and print the epic ID and the subtask IDs.
Then ask two things:

**Which worker runs each fire** — `claude` (default, Opus via the Claude CLI) or
`codex` (OpenAI Codex via `codex exec`). Passed through as `AFK_WORKER`.

**Which orchestration mode:**

- **Attended (default)** — *this thread* becomes the orchestrator. Read
  `.claude/skills/afk-feature/orchestrator.md` and run that loop yourself: spawn
  one `scripts/afk-fire.sh` worker fire per task, watch CI and merge step PRs
  in-thread, advance dex, and open the integration PR at the end. CI waits cost
  no model fire. Requires this Claude Code session to stay open.
- **Unattended** — run `./scripts/afk-loop.sh <epic-id>` (the bash orchestrator).
  Survives without this session; use it for long overnight runs or when the user
  will close the terminal. `AFK_WORKER=codex ./scripts/afk-loop.sh <epic-id>` for
  codex.

Both modes share the same primitives: `afk-fire.sh` is one stateless implement-only
worker fire; the orchestrator (this thread, or the bash loop) handles CI/merge/dex.

Don't start either without confirmation — both consume tokens and write code. Once
the user says go: for **attended**, begin running the orchestrator loop; for
**unattended**, run `afk-loop.sh` in the foreground (they see round output) or
background (`run_in_background: true`) per their preference.

## Re-runnability

Every step above is idempotent:
- Existing branch → switch, don't recreate.
- Existing plan file → don't overwrite.
- Existing epic for this plan (match by root task name) → reuse.
- Existing subtask name under the epic → skip.

If the user re-runs this skill after a partial bootstrap, it picks up where the previous run left off.

## What this skill does NOT do

- It doesn't create a cloud routine. For that, use `/afk-feature` (which writes to a separate worker.md instruction file and uses `RemoteTrigger`).
- It doesn't modify the script. Anything the loop needs (claude/feat-* branch, dex epic in working tree) must be set up before handoff.
- It doesn't decide whether the work is a good idea. If the user picked a bad plan, the loop will faithfully execute a bad plan.
