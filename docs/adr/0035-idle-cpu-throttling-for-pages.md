# ADR 0035 — Pages are CPU-throttled while the app is idle, never hidden

**Status:** Accepted
**Date:** 2026-08-20
**Related:** [docs/pan-zoom-perf-unknowns.md](../pan-zoom-perf-unknowns.md) §1 (the survey that identified `Emulation.setCPUThrottlingRate` as the one lever that works on a visible renderer, and `backgroundThrottling` as the wrong knob)

## Context

Specular keeps many live web pages resident at once, and GPU-heavy pages are
exactly what the app is for. Nothing in the app suppressed any of that work
while the user was in another application, so a canvas of animated pages drained
a battery at full rate whether or not anyone was looking at it.

The reflex fix — `backgroundThrottling` — does not apply. Chromium only
background-throttles renderers it considers **hidden**, and a window that is
visible but unfocused is not hidden. `pan-zoom-perf-unknowns.md` had already
recorded the finding: *"for visible views it changes nothing."* No Electron
preference decides this for us, so the policy has to be ours.

The constraint that shapes the answer is agents. Agents drive this app while
nobody is watching it — that is the normal case, not the edge case. A design
that quiets pages when the window loses focus is a design that breaks agent
work, silently, in the exact conditions agents run in. Viewport culling already
faced this and solved it by exempting `automationInteractivePageCounts`
(`layout-engine.ts`), but that signal is narrower than it looks: it is raised
only when a CDP bridge connects. An agent that creates a page, navigates it, or
screenshots it over HTTP never raises it.

## Decision

### 1. Throttle the guest main thread; never hide the view

Idle pages get `Emulation.setCPUThrottlingRate` over each page's shared
debugger session. rAF-driven work — the expensive kind — collapses, while the
compositor keeps its frame.

Hiding the view (`setVisible(false)`) was the alternative, and it is the more
thorough throttle. It is rejected because `capturePage` is how agents see:
`frame-compositor.ts` and `page-queries.ts` read pixels straight off the page's
`webContents`. A hidden view returns a stale frame or nothing, and
`FrameEvictionManager` keeps only ~5 saved frames, so an unattended screenshot
of the sixth hidden page comes back blank. A throttled page still yields live
pixels. Slowing a page is recoverable in microseconds; blanking it is a
correctness bug in the agent path.

The same reasoning rules out relying on Chromium's own occluded state: it is
window-granular and not app-triggerable per view.

### 2. Two ways to stay awake, because one signal is not enough

- **Agent activity** — a trailing window pulsed by control-server traffic.
  Every authorized request counts except a small deny-list of endpoints an idle
  client polls on a timer (`/mcp/session/ping`, `/session/presence`, the perf
  status routes); without that exclusion, one connected MCP client would hold
  every page awake forever by doing nothing. An unrecognized route counts as
  activity: the failure that costs battery is preferable to the failure that
  breaks an agent mid-task.
- **Awake holds** — a ref-counted claim for work that needs live frames over a
  stretch with no traffic of its own to prove it. Video recording and perf
  tracing take one. A trace of a throttled page measures the throttle.

A page with a live CDP bridge stays exempt on its own, via
`automationInteractivePageCounts`, so a long agent session with no other traffic
does not decay into a throttled page.

### 3. The policy is pure; only the application is not

`page-idle-policy.ts` is a pure function from observed state to a verdict plus
the moment that verdict can next flip. `page-idle-throttle.ts` owns the state,
the single timer, and the CDP dispatch. The rules are unit-testable without
Electron, and the caller has exactly one timer to arm.

## Consequences

- An unfocused app with a canvas of animating pages costs a fraction of what it
  did. The blur grace (5s) means alt-tabbing away and back never engages it.
- Pages are *slowed*, never stopped. A throttled page still makes progress —
  timers fire, loads complete, sockets stay up — just far more cheaply.
- A page whose debugger is owned by an open DevTools frontend cannot be
  throttled. Attach fails, the page keeps running at full speed, and the next
  evaluation retries. Correct, and the same trade-off `page-color-scheme.ts`
  already makes.
- `SPECULAR_DISABLE_IDLE_THROTTLE=1` turns the whole thing off, for diagnosing a
  page that misbehaves under throttling.

## Alternatives rejected

- **`backgroundThrottling: false` / `true`** — governs hidden renderers only.
  Irrelevant to an unfocused-but-visible window, which is the whole problem.
- **`setVisible(false)` on cull** — blanks the agent screenshot path (§1). It
  remains the right answer for a future "parked page" tier, but only paired with
  our own captured bitmap, which is a larger design.
- **Throttle only culled (off-canvas) pages** — cheaper to reason about, but
  leaves the common case untouched: a visible canvas of animating pages behind
  another app is exactly what drains the battery.
