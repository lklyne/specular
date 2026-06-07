# Self-healing CLI loop

A local loop that continuously improves the `specular` CLI: a fresh-context agent
uses the CLI to edit the canvas, measures how awkward that was against concrete
friction signals, fixes a bug or smooths the surface, and records the outcome in
git. Built to generalize to other domains (wireframing, design) by swapping a pack
of docs.

## Shape: a general engine + a domain pack

Two layers. The **engine knows nothing about the CLI**; a **pack of docs aims it.**

```
harness/                 # general purpose — never mentions the CLI
  loop.sh                # while-loop; takes a pack dir; one fresh `claude -p` per round
  fire.md                # generic prompt template (only substitution: the pack path)

packs/cli/               # aims the engine at the CLI
  charter.md             # what "better" means; mechanical friction signals; workflows
  guardrails.md          # branch-only, don't game probes, flag deletions, skill sync
  backlog.md             # the memory: ## Todo / ## Done (git history = the record)
  probes.md              # how to run the probes + coverage map + gaps

tests/smoke/cli/         # the runnable probes (reuse the existing smoke harness)
  cli-probe-utils.ts     # runCli(): built CLI → ephemeral smoke app, returns code/stdout/stderr/json
  *.probe.test.ts        # assert agent-friendliness as fact
```

Run it: `./harness/loop.sh packs/cli`. A second domain is a second pack
(`packs/wireframe/…`) + its probe glob — the engine is unchanged.

## The loop (one fire = one unit of work)

Each round is a fresh `claude -p` (no `-c`): no context accumulates, cost per round
is flat, and the only durable state is git. A fire:

1. Reads the pack (charter → guardrails → backlog → probes) + CLAUDE.md.
2. **Heal preempts improve.** Runs the probe command + `typecheck`/`test:unit`. If
   red, the only job is making it green.
3. Else takes the single top `## Todo` item. Smallest change; prefer deletion.
4. Verifies (typecheck + unit + probes all green; never weaken a probe to pass).
5. Records: moves the item to `## Done` (date + sha), appends new friction as
   `## Todo`, commits code + backlog together.

## The probes: agent-friendliness as assertions

Probes exercise the **real CLI binary** (not the HTTP client) against the
**ephemeral smoke app** (`tests/smoke/global-setup.ts` already boots an isolated,
throwaway instance — temp user-data, private discovery file, random port). The real
app is never touched, so the flaky-reboot problem is sidestepped entirely.

`runCli()` points the CLI at the smoke instance via `SPECULAR_DISCOVERY_FILE` and
returns `{ code, stdout, stderr, json }`. Assertions encode the charter's
**mechanical friction signals** — parseable stdout, actionable stderr, non-zero
exit on misuse, edits observable on read-back, call count per intent. "Looking for
improvements" is concrete: a friction note becomes a probe asserting the better
behavior, the probe goes red, the fire fixes the CLI, the probe goes green.

## Guardrails (the dumb safety model)

- **Branch only, never main.** The loop refuses to run on `main`/`master`. Commits
  land on a review branch; no auto-merge. That review gate *is* the safety rail.
- **Can't game the metric.** Weakening/deleting a probe or deleting a feature is a
  `REVIEW:` note for a human, never something a fire does.
- **Skill stays in sync.** CLI behavior changes update both skill copies (per
  CLAUDE.md); a skill edit must keep probes green.

## Tokens

Bash loop = 0 tokens. Default fire = Sonnet (most fixes). Run the same script with
`MODEL=opus` occasionally for a deeper simplification pass. No per-task routing to
build.

## Scope: v1 vs v2

**v1 (this commit).** Engine + CLI pack + probe harness + two starter probes +
seeded backlog. History of record = git (backlog.md diffs + commits). Verification
runs against the throwaway smoke app.

**v2 — artifacts in the real Specular.** Because probes run against the ephemeral
smoke app, the canvas they build is discarded, so the loop's progress isn't visible
in the user's live app. v2 adds a read-only projection step that points the CLI at
the *canonical* discovery file (the running app) and renders `backlog.md` Done/Todo
entries as canvas notes — dogfooding the CLI to display its own history. Deferred so
v1 stays simple; it's additive and changes nothing above.

## First real iterations (seeded in packs/cli/backlog.md)

1. Reliable `specular dev restart` (idempotent, cross-platform) — the highest-
   leverage agent-friendliness fix and the cause of "the agent can't reboot the app."
2. `create page` should reject bare paths with a full-URL message.
3. `snapshot` output should be parseable without a flag (or document it).

## How to run

```
git switch claude/self-healing-cli-loop-FmZXa   # a review branch, not main
pnpm build:cli && pnpm test:smoke -- cli         # sanity-check the probes
./harness/loop.sh packs/cli                      # MODEL=opus / MAX_ROUNDS=N / SLEEP=N to tune
```
