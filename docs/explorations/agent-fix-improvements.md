# Agent-fix workflow — audit summary & proposed improvements

Summary of an audit of the in-app fix agent (`src/main/agent-fix/`) and the
comment-thread UX around it, plus the design exploration that followed.
Companion artifacts: the wireframes in
[`comment-panel-wireframes/`](./comment-panel-wireframes/README.md).

## What exists today (and what holds up)

A comment thread maps 1:1 to a resumable Claude session: the first fix spawns
`claude -p` with a rich prompt (source location, React ancestry, live-page
inspection via the specular CLI), stores the returned `fixSessionId` on the
annotation, and each user reply resumes that session with just the new
message. Stale sessions fall back to a fresh run with full context.

**Keep all of this.** Thread-per-session is the right unit; the live-page
inspection loop (edit → hot reload → agent looks at the page) is the
product's differentiator; `<<RESOLVE>>` stays a hint and the user always
resolves; the per-annotation in-flight lock, reply cap, and resume fallback
are sound.

## Trust & control gaps (ranked)

1. **Diff + revert per run.** The agent edits the working tree with no diff
   surfaced, no checkpoint, no revert. Capture a git ref before each run,
   show `git diff --stat` (expandable) on completion, one-click revert.
   The single highest-leverage fix.
2. **Cancel.** A running fix can't be stopped — only a 10-minute SIGKILL
   timeout. Wire a Stop button to the process (or `interrupt()` with the
   Agent SDK, below).
3. **Per-repo serial queue.** Concurrent fixes share one working tree and
   one hot-reloading dev server; "fix all" + auto-fix makes collisions
   common, not rare. Worktrees would break the live-preview loop, so
   serialize per repo with a visible "queued behind N" state. Also makes
   per-run diffs attributable.
4. **Cross-thread awareness.** Sessions are isolated; agent B doesn't know
   agent A just changed the same file. Mitigations: list other unresolved
   comments for the origin in the prompt (awareness only), and make
   "fix all" one session handling the batch instead of N parallel ones.
5. **Every reply triggers a run** when auto-fix is on — including "thanks!".
   Resolved by the composer design (below).
6. **Small stuff.** `claude-${model}-4-6` hardcodes a version that will rot
   (use bare aliases); permissions are binary default/dangerously with no
   middle ground; the orchestrator has no integration test (only the three
   leaf units do).

## UX: comment threads move to the right panel

The thread popover's jank is structural, not polish: it's a floating window
glued to a moving spatial coordinate (live-bbox subscriptions that go stale,
element-attachment drift correction, clamping, repositioning on every
pan/zoom/scroll), it dies on any outside click, and it caps a multi-minute
agent conversation at 320px. Decision direction: **move threads wholly into
the right panel as a consistent chat interface** — this deletes the
positioning stack, the click-catcher, and the interaction-layer carve-outs
entirely, and gives agent runs a persistent streaming surface.

What stays spatial: badges as entry points, the inline creation composer
(spatial context matters most at creation), anchor highlight + scroll-reveal
when a thread is selected, bidirectional hover linking.

The six wireframe docs front-load the open decisions:

| Doc | Decision | Leaning |
|---|---|---|
| 01 Panel architecture | Selection-driven panel vs persistent chat: tabs / takeover / docked split | — |
| 02 List ↔ thread nav | Drill-in / accordion / docked thread | Drill-in (only one giving the chat full height) |
| 03 Thread anatomy | Header, context card, run block, diff chip, resolve suggestion, composer | — |
| 04 Run lifecycle | Queued / running (Stop) / completed (diff, revert) / failed / resume-fallback | Mid-run replies queue as follow-ups |
| 05 Composer | Does Send trigger the agent: always / two buttons / toggle chip | Toggle chip (one button, cost visible, doubles as manual trigger) |
| 06 Choreography | What stays on canvas | Badges + inline composer stay; panel owns conversation |

This partially supersedes the popover leg of ADR 0006 / the overlay framing
of ADR 0002 for annotations — write a short ADR when the direction is
confirmed.

## Runtime: swap the headless spawn for the Claude Agent SDK

Not previously explored. Recommendation: **yes**, staged under the existing
`invokeClaude` seam (`_setSpawnerOverride` + `FixResult` mean the
orchestrator and tests don't change).

What it buys, mapped to the gaps above:

- `interrupt()` → the Stop button (gap 2).
- `canUseTool` callback → the permission middle ground (gap 6): auto-allow
  edits within the bound repo, surface everything else as an approval card
  in the thread — per-action consent instead of the "dangerously" checkbox.
- `PostToolUse` hooks → structured file-edit events for diff attribution
  (gap 1; git checkpoint still needed for revert).
- `includePartialMessages` → token-level streaming into chat bubbles.
- Typed message stream → delete `stream-json-parser.ts`; `resume`/`fork`
  first-class; bundled Claude Code binary → consistent behavior independent
  of the user's installed CLI (and fixes the model-version rot).

Gotchas: set `systemPrompt: {type: 'preset', preset: 'claude_code'}` (SDK
default is minimal); pin `settingSources` explicitly (the fix prompt depends
on the user-level specular skill); Electron packaging of the platform binary
(tens of MB, forge unpack work); version coupling flips from
"drifts with user's CLI" to "pinned, shipped deliberately". Auth is
unchanged (`~/.claude` credentials).

## Suggested sequencing

1. Diff + revert per fix (trust)
2. Cancel (trust) — trivial once on the SDK
3. Per-repo serial queue (correctness)
4. Thread view in the right panel; SDK swap lands *underneath* it, since
   approval cards and streaming bubbles shape the thread's message types
5. Batch "fix all" as one session + cross-thread awareness in the prompt
6. Composer agent-toggle, model alias fix, orchestrator integration test
7. ADRs: panel-hosted threads; fix agent on the Agent SDK
