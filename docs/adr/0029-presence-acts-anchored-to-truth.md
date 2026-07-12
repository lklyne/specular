# ADR 0029 — Presence acts are anchored to truth

**Status:** Accepted
**Date:** 2026-07-11
**Related:** issue #318 (browse robustness / CLI unification), issue #319 (presence cursor realism), [ADR 0018 — Cloud sync and canvas sharing](./0018-cloud-sync-and-canvas-sharing.md) (agent-as-peer presence attribution).

## Context

Agent presence cursors should read as a real collaborator: the cursor
travels to a target, pauses, and *then* the page reacts. At the same time
the CLI must stay fast and hiccup-free for the agent driving it, and the
gap between CLI commands is unknowable (LLM thinking time). These goals
have been re-derived and re-litigated across several build attempts, so
the resolution is recorded here.

The constraint that forces a decision is a trilemma. Pick two of:

1. **Live page** — the webview is real; the page reacts the instant an
   input event dispatches. Its reaction cannot be delayed or replayed.
2. **Move-then-act** — the cursor visibly arrives at the target before
   the effect happens.
3. **Zero blocking** — input commands forward to the page immediately.

The tempting third way — dispatch immediately and animate the cursor
afterward ("retro-performance") — violates visible causality: the menu
opens while the cursor is still mid-flight. On a live surface that reads
as broken, not realistic. There is no buffering trick that avoids this,
because pixels come from a live webview, not from a replayable stream.

## Decision

**Pay a bounded pre-act dwell; anchor every act to real dispatch time;
never retro-animate causality.**

1. **Bounded pre-act dwell.** Mutating input events (mouse, scroll)
   intercepted by the CDP proxy wait `max(0, STEP_DELAY − elapsed since
   the cursor's last reposition)` before dispatching
   (`waitForPresenceDwell` in `app-control-server.ts`, budget constants
   in `src/shared/presence-timing.ts`). The wait is a plain timer with no
   external dependency: it can add bounded latency, it can never hang.
   Reads and observations (snapshot, get, wait, eval) are never delayed.

2. **Acts anchor to truth.** The visual "act" moment (click ripple,
   label emphasis) coincides with the actual dispatch, which happens
   *after* the dwell. The CLI's success result returns only after real
   dispatch. Consequently the agent's view of the world is never ahead
   of or behind reality, and ordering between the CLI and the page
   cannot desync — the dwell is the single, inline synchronization
   point.

3. **Amortize the dwell toward zero with intents, not by shrinking the
   guarantee.** The CLI fires a non-blocking intent before spawning
   agent-browser; when the target is resolvable (CSS selector, `text=`
   locator, `find role|testid` — resolved in the background via
   `findPresenceTarget` with stale-intent guards, shipped with #318),
   the cursor starts traveling during time the CLI was spending anyway,
   and the residual dwell at dispatch approaches zero. Opaque
   `@eN`-ref targets cannot pre-travel this way and pay more of the
   dwell; narrowing that gap (proxy-level pre-move on box-model
   responses, adaptive dwell) is #319's work and changes the *amount*
   paid, never the anchor rule.

4. **Never retro-animate.** No cursor movement is synthesized after an
   effect to make it look intended, and no speculative movement is made
   toward predicted targets. When presence cannot honestly depict an
   action (see JS-driven mutations below), it shows an honest label and
   stays put.

## Consequences

- **Blocking exists and is owned.** Worst case one `STEP_DELAY` per
  mutating input event; typically far less on the selector path. This
  is noise against agent-browser round-trips and LLM turns, and it is
  deterministic. "Never block the CLI" is honestly restated as:
  *bounded, adaptive blocking only on mutating input events, amortized
  toward zero by pre-travel.*
- **The renderer must finish travel within the dwell.** Renderer travel
  duration is capped at or below the server's dwell budget (the
  `syncCapMs` idea in `cursor-tuning.ts`). Any motion-model change must
  preserve this cap or the act fires mid-flight.
- **JS-driven mutations are outside the guarantee.** `eval`-style
  mutations bypass `Input.dispatchMouseEvent`, so the page can change
  with the cursor elsewhere. Per rule 4 the cursor shows an honest
  activity label and does not fake movement; the skill steers agents
  toward `click`/`fill` for anything a user watches.
- **The dwell widens a coordinate-staleness race.** Agent-browser
  resolves element coordinates, then the dwell delays dispatch —
  stretching the resolve→dispatch gap from ~10ms to up to the full
  dwell. On animating or hot-reloading pages the element can move in
  that window. This is a named, accepted cost; adaptive dwell keeps the
  window small in bursts, and it is tracked with an agent-test scenario
  (#319) rather than hidden.

## Future path: presence event timeline

If burst pacing, label stability, or ripple timing accumulate per-case
patches — or when presence must travel over network sync (ADR 0018's
agent-as-peer) — the state-cell contract (`PresenceCursorEntry` as
latest-state) evolves into a per-session **event timeline**: main
appends `(t, kind, target)` events with kinds `travel-to | dwell | act |
label | ambient`; the renderer plays them behind a small latency buffer
with minimum display durations.

The anchor rule survives the migration and constrains it: **`act`
events are pinned to real dispatch time and are never buffered** (the
pre-act dwell already reserves that window server-side, so buffer ≤
dwell); only travel/label/ambient events are smoothed. A naive uniform
buffer would reintroduce the retro-performance causality break through
the back door — this is the reason the timeline cannot be adopted as a
generic "replay everything late" layer.

An event log is also a recording: replaying captured sessions in the
debug playground is how "feels good to watch" becomes reviewable
instead of anecdotal. Interim implementations should keep choreography
helpers pure and event-shaped (`(t, kind, target)` inputs, not
`PresenceCursorEntry` field reads) so they port unchanged.
