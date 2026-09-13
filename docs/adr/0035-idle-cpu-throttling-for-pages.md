# ADR 0035 — Pages are frozen while the app is idle, never hidden

**Status:** Accepted (mechanism revised 2026-08-22 — see *Postmortem: CPU throttling*; revised again 2026-09-12 for offscreen pages — see *Postmortem: lifecycle freeze on offscreen pages*)
**Date:** 2026-08-20
**Related:** [docs/pan-zoom-perf-unknowns.md](../pan-zoom-perf-unknowns.md) §1 (the survey that identified `backgroundThrottling` as the wrong knob)

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

### 1. Freeze the page; never hide the view

> **Superseded for offscreen pages (ADR 0038).** The lifecycle freeze below
> was written for native `WebContentsView` pages. Once every page became a
> hidden offscreen `BrowserWindow`, thawing stopped working — see *Postmortem:
> lifecycle freeze on offscreen pages*. The current lever is the host's frame
> rate (`PageHost.setIdle`); the policy (§2, §3) is unchanged.

Idle pages get `Page.setWebLifecycleState({ state: 'frozen' })` over each
page's shared debugger session, and `'active'` when the app wakes. A frozen
page's task queues stop — timers, rAF, script — while the compositor keeps its
last frame. Holding the state costs nothing: measured on a real page, a frozen
renderer sits at ~1 idle wakeup/s and 0% CPU, and `capturePage` still returns
a full, non-empty frame.

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

Inline HTML files are not pages: they render as cross-origin iframes inside
canvas-bg, in renderers of their own that no page-level lever reaches. The
verdict travels to canvas-bg as the `idle` runtime slice and the renderer
hides the iframe (`visibility: hidden`) — Chromium gives a frame it is not
rendering no rAF at all, the same thing that already quiets one scrolled
off-screen. Measured: a 120fps iframe drops to 0 either way.

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
the single timer, and the dispatch to each host. The rules are unit-testable without
Electron, and the caller has exactly one timer to arm.

## Consequences

- An unfocused app with a canvas of animating pages costs a fraction of what it
  did. The blur grace (5s) means alt-tabbing away and back never engages it.
- Pages are *stopped*, not slowed. A frozen page makes no progress until the
  app wakes: timers do not fire and script does not run. A page with a load in
  flight is exempt so loads still complete; a site holding a socket may
  reconnect on resume, the same as it does after a laptop sleeps.
- A page whose debugger is owned by an open DevTools frontend cannot be
  frozen. Attach fails, the page keeps running at full speed, and the next
  evaluation retries. Correct, and the same trade-off `page-color-scheme.ts`
  already makes.
- There is no global off switch. Every exemption — focus, awake holds, agent
  traffic, a live CDP bridge, a load in flight — is derived from observed
  state, so a page that must stay fast has a reason the policy can read. A
  kill switch would be the one input nothing observes, and a second code path
  to measure. If throttling is ever wrong for some page, the fix is a rule
  here, not a flag around the whole thing.

## Postmortem: CPU throttling

The first version of this decision used `Emulation.setCPUThrottlingRate`
(rate 20) instead of freezing, on the reasoning that a slowed page still makes
progress. It shipped, and a simple canvas of 15 idle pages went to ~86% CPU
and ~6,900 idle wakeups/s — worse than no throttle at all.

The cause is how Chromium implements the throttle on POSIX
([thread_cpu_throttler.cc](https://github.com/chromium/chromium/blob/main/third_party/blink/renderer/platform/scheduler/common/thread_cpu_throttler.cc)):
a dedicated `CPUThrottlingThread` sends `SIGUSR2` to the renderer main thread
every 200µs, forever, and the signal handler busy-spins for its share of the
quantum rather than sleeping. The cadence is fixed, so the rate does not
change the cost. Measured in a throwaway Electron harness on `example.com`:

| state | idle wakeups/s | CPU | rAF/s |
|---|---|---|---|
| visible, no override | 15 | 0.0% | 60 |
| visible, throttle rate 20 | 3,949 | 6.1% | ~35 |
| culled (0×0), no override | 1.3 | 0.0% | ~0 |
| culled, throttle rate 20 | 3,672 | 5.8% | ~0 |
| culled, throttle rate 2 | 3,924 | 3.9% | ~0 |
| visible, lifecycle frozen | 10 | 0.0% | 0 |

Two things worth keeping from this: a culled page is already quiet on its own,
so any idle mechanism is only buying something on *visible* pages; and idle
wakeups, not CPU%, are the number to watch for battery — the throttle looked
like a win on CPU% in the first screenshot and was a loss on wakeups by three
orders of magnitude.

## Alternatives rejected

- **`Emulation.setCPUThrottlingRate`** — see the postmortem above. A fixed
  200µs signal loop per throttled renderer; ~4k wakeups/s and ~6% CPU each.

- **`backgroundThrottling: false` / `true`** — governs hidden renderers only.
  Irrelevant to an unfocused-but-visible window, which is the whole problem.
- **`setVisible(false)` on cull** — blanks the agent screenshot path (§1). It
  remains the right answer for a future "parked page" tier, but only paired with
  our own captured bitmap, which is a larger design.
- **Throttle only culled (off-canvas) pages** — cheaper to reason about, but
  leaves the common case untouched: a visible canvas of animating pages behind
  another app is exactly what drains the battery.

## Postmortem: lifecycle freeze on offscreen pages (2026-09-12)

After ADR 0038 moved every page into a hidden offscreen `BrowserWindow`, shader
and rAF animations ran only on the page the user had clicked into. They stopped
the moment selection moved off it and stayed stopped when the user came back.
Reloading that one page brought it back; nothing else did.

**Mechanism.** `Page.setWebLifecycleState('frozen')` is implemented browser-side
as `WasHidden()` + `SetPageFrozen(true)`; `'active'` as `SetPageFrozen(false)` +
`WasShown()`. On a window that has never been shown, the show half does not
restore compositor visibility. Measured in a bare Electron 43 offscreen
shared-texture window with a rAF counter page (1.2 s windows):

| Step | paints | rAF ticks | `document.visibilityState` |
|---|---|---|---|
| healthy | 72 | 72 | visible |
| `frozen`, then `active` | 0 | 0 | visible |
| then `Emulation.setFocusEmulationEnabled(true)` | 72 | 72 | visible |
| then `setFocusEmulationEnabled(false)` | 0 | 0 | visible |
| any reload or navigation after a thaw | 0 | 0 | **hidden** |

A plain hidden `show: false` window with no offscreen rendering fails the same
way, so this is Chromium's hidden-window visibility bookkeeping, not an OSR
bug. It never surfaced under the native-view architecture because the
device-emulation re-raster, which ADR 0038 deleted, happened to nudge thawed
pages back to life on every zoom settle.

Focus emulation (`page-focus-emulation.ts`) is enabled only on the
keyboard-target page. That is what gave the symptom its shape. The entered
page ran because emulation was on. Leaving it turned emulation off, which
stopped the page. A reload re-applied emulation to the target page, so it
looked like a fix.

Nothing recovers a page for good once it has been frozen. I tried `invalidate`,
`stopPainting`/`startPainting`, debugger detach, resize,
`Emulation.setDeviceMetricsOverride`, `Page.bringToFront`, a
`setBackgroundThrottling` toggle, and a host window shown at opacity zero.
Showing and re-hiding the window revives the current document, but every later
document loads hidden. So a nudge loop over visible pages is not an option
either.

**Replacement lever: the host's frame rate.** `webContents.setFrameRate(1)`
throttles the offscreen compositor's BeginFrame cadence, which is what paces
`requestAnimationFrame`, so it quiets a page's own animation loop and not just
texture delivery. Timers keep running. It restores immediately and survives
cross-process navigation:

| State | paints/s | rAF/s | timer ticks/s |
|---|---|---|---|
| normal | 60 | 60 | 20 |
| `setFrameRate(1)` | 2 | ~2 | 20 |
| back to `setFrameRate(60)` | 60 | 60 | 20 |
| after cross-site navigation | 60 | 60 | 20 |

Paired with `stopPainting()`, the idle page produces no textures at all.
`PageHost.setIdle` owns both, and composes with viewport culling as
`painting && !idle` so a culled page stays culled across an idle cycle.

Also rejected: CDP `Emulation.setVirtualTimePolicy({ policy: 'pause' })` stops
timers but not rAF, and replays every missed timer in a burst on resume.

**Cost accepted.** A throttled page is slowed, not stopped. Script and timers
run at full rate and rAF at about 2/s, so the 0% CPU the lifecycle freeze
delivered is gone. That is the trade for pages that come back. Do not send
`Page.setWebLifecycleState` to a page host.
