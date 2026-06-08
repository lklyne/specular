# CLI self-heal charter

This pack aims the self-improvement engine (`harness/loop.sh`) at the Specular
CLI. The engine is general purpose; everything CLI-specific lives in this folder.

## Goal

Make the `specular` CLI **more agent-friendly** by fixing bugs, simplifying the
architecture behind it, and smoothing the surface an agent actually touches. The
CLI is how agents edit the canvas (`src/main/cli.ts`, `src/main/cli-commands.ts`,
HTTP routes in `src/main/routes/`).

## What "better" means — mechanical friction signals

"Easy to use" is not a feeling; it is measurable. When you run a prescribed
workflow (below), treat any of these as friction worth a backlog item or a probe:

- **Call count** — one logical intent should take ~one command. Needing 3 calls to
  do one thing is friction.
- **Parseable output** — a command's result should be valid JSON on stdout with no
  extra flag required. If you had to pass `--format` or post-process to parse it,
  that's friction.
- **Actionable errors** — a failure must exit non-zero and say *what to do* on
  stderr, not just what broke. Errors must never land on stdout.
- **No source-diving** — if you had to read `src/main/` to discover an argument or
  flag, the help/skill is inadequate.
- **No guessed flags** — if the flag you needed isn't in the skill
  (`.claude/skills/specular/SKILL.md`), that's a documentation bug, not your fault.

The durable form of any of these is a probe under `tests/smoke/cli/` that asserts
the fixed behavior. Freeform notes are only candidates; convert the good ones.

## Prescribed canvas workflows

Run these against the ephemeral smoke app (the probes already do — see probes.md).
Do not invent your own task to grade; run these, so the assessment measures a
fixed thing instead of a flattering one.

1. Build a small canvas: create two pages at breakpoints + a note, then read the
   workspace back and confirm the edits are observable.
2. Annotate a page, then resolve the annotation.
3. Group several pages, auto-layout them, then focus the group.

For each: count the calls, check every output parses, check every error is
actionable. File friction as Todo items in backlog.md.

## Probe command

```
pnpm build:cli && pnpm test:smoke -- cli
```

`pnpm build:cli` rebuilds `out/main/cli.js` so probes exercise your latest change.
`-- cli` filters vitest to `tests/smoke/cli/`.

## Docs are a maintained surface

The CLI's agent-friendliness includes the docs that describe it. They must match
observed behavior — treat them as part of "done" for any CLI change:

- `CLAUDE.md` → the `## Specular CLI` and `## Agent integration` sections.
- `.claude/skills/specular/SKILL.md` and `resources/skills/specular/SKILL.md`
  (update both in the same commit — see guardrails.md).

When a command's behavior, flags, or arguments change, update these in the same
commit. When you notice a doc claims something the CLI doesn't do (a missing/renamed
command, a wrong flag or argument), that is a heal item: fix it if small, else file
it in backlog.md.

## Non-goals (do not drift here)

- Renaming variables or reflowing code with no behavior or ergonomics change.
- New CLI features that aren't on the backlog — file them, don't build them.
- Touching the renderer or canvas UI. This pack is the CLI surface only.
- Making a probe pass by weakening it. See guardrails.md.
