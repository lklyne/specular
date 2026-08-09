# Interaction Layer — Architecture Spec

*Draft for `docs/interaction-layer.md`*

---

## 1. Philosophy

**The platform's honest signals become our explicit invariants.**

Electron gives us a small, opinionated set of primitives for stacking and input (`addChildView`, `setVisible`, `setBounds`, native focus routing). Every friction we've hit — undo breaking routing, bounds-hiding overlays, drop dedup, focus-stealing after load — is the platform telling us what shape the architecture wants to be. This spec codifies that shape instead of working around it per-feature.

Three commitments shape every decision below:

1. **Boundaries are expensive.** A WCV boundary costs ~70 MB, an IPC bridge, a preload surface, and a coordination problem. We add them only when a failure mode genuinely requires them.
2. **Input has one authority.** At any moment, exactly one place decides "what does this mouse event mean" — not five coordinated listeners.
3. **State changes happen at known moments.** View-stack, visibility, and bounds mutations happen inside one scheduled layout pass, never during event dispatch.

If a future change violates any of these, the change is wrong for this system, not the system wrong for the change.

---

## 2. Goals & Non-Goals

### Goals

- **Predictable gesture routing.** A user's pointer action maps to exactly one gesture, chosen by one arbiter.
- **Minimal stacking surface.** Three WCVs in the stacking region (bgView, aboveView, liveViews). Everything else is DOM composition.
- **Cheap idle state.** N frames on canvas ≠ N live renderer processes.
- **Structural testability.** Every gesture has a begin/update/commit/cancel path that can be exercised without Electron.
- **Agent-legible.** Canvas state is readable and mutable by CLI/HTTP without replaying mouse events.

### Non-Goals

- Supporting every combination of nested overlays we've ever experimented with. The architecture deliberately narrows what's expressible.
- Per-overlay process isolation as a general value. We keep isolation only where it actually buys crash containment (pages, DevTools) or independent lifecycle (toolbar, sidebar).
- Pixel-perfect parity with legacy interaction quirks that stemmed from accidental overlay ordering.
- A full offscreen-texture compositor for live pages. That's a separate track (`docs/offscreen-rendering-research.md`).

---

## 3. System Overview

### 3.1 The three planes

```
┌──────────────────────────────────────────────────────────┐
│  toolbar       ← own WCV (long-lived, independent UI)    │
├──────────────────────────────────────────────────────────┤
│  sidebar       ← own WCV                                 │
├──────────────────────────────────────────────────────────┤
│  devtools      ← own WCV (Chromium requirement)          │
├──────────────────────────────────────────────────────────┤
│  ┌────────────────────────────────────────────────────┐  │
│  │ ABOVE-PAGES PLANE                                  │  │
│  │   aboveView (single WCV) — canvas-mode keyboard    │  │
│  │   ├─ input gate (canvas gesture capture)           │  │
│  │   ├─ selection / marquee / drag visuals            │  │
│  │   ├─ entity bodies (sticky, shape, file, image,    │  │
│  │   │     video, markdown, component)                │  │
│  │   ├─ edges + anchor dots                           │  │
│  │   ├─ group bounds                                  │  │
│  │   ├─ selection outlines + resize handles           │  │
│  │   ├─ keyboard-target focus ring + agent halo       │  │
│  │   ├─ canvas-anchored popups (CanvasItemPopup)     │  │
│  │   └─ comments, annotations, floating UI            │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ LIVE PAGES (0-N WCVs)                              │  │
│  │   One per active frame: selected + scroll peers    │  │
│  │   Inactive frames rendered as bitmaps into bgView  │  │
│  ├────────────────────────────────────────────────────┤  │
│  │ BELOW-PAGES PLANE                                  │  │
│  │   bgView (single WCV)                              │  │
│  │   ├─ canvas grid, camera, pan/zoom transform       │  │
│  │   ├─ frame borders + device shells                 │  │
│  │   └─ bitmap compositor for inactive pages (future) │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘

     ┌──────────────────────────────────────────────┐
     │ cursorOverlayWindow  ← child BrowserWindow   │
     │   (sibling of win; NOT in the WCV stack)     │
     │   transparent, frameless, focusable:false,   │
     │   setIgnoreMouseEvents(true, forward:false)  │
     │   └─ agent-presence cursors only             │
     └──────────────────────────────────────────────┘
```

The cursor overlay is deliberately outside the three-plane model. It sits in its own OS-level child window because Electron 40's `WebContentsView` has no `setIgnoreMouseEvents` (see §7.3); a WCV would block native frame input. It is paint-only, never captures events, and is the single sanctioned exception to "all canvas-region rendering lives in one of the three planes."

### 3.2 What each plane owns

| Plane | Owns visuals | Owns input | Number of WCVs |
|---|---|---|---|
| `bgView` | Canvas grid + camera transform + frame borders/device shells + (future) inactive-page bitmaps | Nothing (always `setVisible(true)`; never holds keyboard focus post-migration) | 1 |
| `liveViews` | Active web content | Native page input + keyboard, only while the `shouldFocusSelectedPage` predicate elects a single page as keyboard target | 0-N |
| `aboveView` | Entity bodies, edges, group bounds, selection outlines + resize handles, focus ring, agent halo, canvas-anchored popups, marquee + drag visuals, comments / annotations / floating UI | All canvas-level pointer input + canvas-mode keyboard (default `FocusTarget`) | 1 |
| `toolbar` / `sidebar` / `devtools` | Their own UI | Their own UI | 1 each |
| `cursorOverlayWindow` | Agent-presence cursors (paint-only, shown only while `win` is focused) | Nothing — `setIgnoreMouseEvents(true)` | Not a WCV; sibling child `BrowserWindow` of `win` |

### 3.3 State authority

Main process owns all workspace state via the two-layer model (Y.Doc + runtime variables) described in `src/main/runtime/CLAUDE.md`. The interaction layer adds three main-process singletons:

- **`InteractionController`** — the gesture state machine.
- **`FocusReconciler`** — decides which webContents holds focus.
- **`DropOwner`** — decides which WCV is the drop target right now.

Renderers are clients. They observe state via IPC broadcasts and send intent messages. They never hold authoritative interaction state.

---

## 4. Subsystem Specs

### 4.1 `InteractionController` (main)

The single arbiter for canvas-level gestures.

```ts
type InteractionMode =
  | { kind: 'idle' }
  | { kind: 'panning' }
  | { kind: 'marquee', origin: CanvasPoint, current: CanvasPoint }
  | { kind: 'dragging-entities', ids: string[], anchor: CanvasPoint }
  | { kind: 'resizing-entity', id: string, edge: ResizeEdge }
  | { kind: 'dragging-edge', from: EdgeEndpoint, target: EdgeEndpoint | null }
  | { kind: 'editing-entity', id: string }

interface InteractionController {
  peek(): InteractionMode
  tryEnter(mode: InteractionMode): Token | InteractionRefused
  update(token: Token, delta: DragDelta): void
  commit(token: Token): void
  cancel(token: Token, reason: 'blur' | 'escape' | 'undo' | 'tab-switch' | 'external'): void
  cancelActive(reason: 'undo' | 'tab-switch' | 'blur' | 'external'): void
  subscribe(listener: (m: InteractionMode) => void): Unsubscribe
}
```

**Rules:**
- Only one non-idle mode at a time. `tryEnter` returns `InteractionRefused` when conflicting — callers **must** handle refusal; refusal is not warned, it is returned.
- Every `tryEnter` that succeeds must pair with exactly one `commit`, `cancel(token)`, or be terminated by a blessed `cancelActive` call. Tokens expire if never closed within N seconds → forced `cancelActive('external')`.
- `cancel` is idempotent and always safe.
- Mode transitions broadcast via IPC to `aboveView` and `bgView` renderers for visual sync.
- `subscribe` listeners fire **after** the state transition, never during event dispatch.

**`cancelActive(reason)` — the external-interrupter back door.**

Some cancel paths don't own the token: the undo observer, tab switches, window blur, and app-wide escape handlers fire from code that didn't start the gesture. They use `cancelActive(reason)` which terminates whatever gesture is currently active, bypassing token ownership. This is the **only** sanctioned way to end a gesture without its token.

Legitimate callers of `cancelActive`, exhaustively:
- Undo/redo observer (runtime reverse-sync) — `reason: 'undo'`.
- Tab switch / workspace switch — `reason: 'tab-switch'`.
- OS window blur (when the app itself loses focus) — `reason: 'blur'`.
- Token expiry timer (internal to the controller) — `reason: 'external'`.

Any other site that feels it needs `cancelActive` is almost certainly a gesture producer that should be calling `cancel(token, reason)` with its own token. Grep discipline applies.

**Why a controller, not a flag:** tokens prevent orphan state when a renderer crashes or a blur storm interrupts a drag. The controller holds the single truth; no one else derives it.

**Inline editing (`editing-entity`).** Every editable canvas item — sticky/text bodies, shape labels, group rename labels — funnels through one mode and one IPC vocabulary:

```
canvas-request-entity-edit { entityId }   // dblclick / shortcut → enter
canvas-commit-entity-edit                  // textarea blur, outside-click
canvas-cancel-entity-edit                  // Escape inside the editor
```

Main is the sole token holder (`src/main/runtime/editing-entity-runtime.ts`). The runtime variable `editingEntityId` is derived from `interactionState` so it follows automatically when an external `cancelActive` (undo / tab-switch / blur) interrupts. Renderers mount the editable surface iff `editingEntityId === myId` — there is no "render textarea on selection" fallback; the read-only view shows otherwise. The pointer router commits on outside-click and **swallows the click** (per [ADR 0001](./adr/0001-click-to-enter-frame-focus.md) precedent: the exit click does not double as the next interaction); a drag attempt on the editing entity is refused silently because `editing-entity` is mutually exclusive with `dragging-entities`.

### 4.2 Input gate (`aboveView`)

The input gate is not a separate WCV — it's a *behavior* of `aboveView`. When any canvas gesture is active or available, `aboveView` is `setVisible(true)` and captures all pointer events in the canvas region. When the user should interact with page content directly, `aboveView` is `setVisible(false)` and native page input works.

**Visibility predicate (single source of truth, post-aboveView migration):**

```ts
function shouldGateBeOpen(s: AppState): boolean {
  // Inspect drives feedback off the page's webContents mousemove (eyedropper);
  // close the gate so the page receives native events.
  if (s.activeTool.kind === 'inspect') return false
  // aboveView is always-on in all other cases.
  return true
}
```

Evaluated inside `layoutAllViews()`. `aboveView.setVisible(shouldGateBeOpen(state))` is the only call that toggles it. See `src/main/runtime/gate-predicate.ts`.

The OR-chain from earlier drafts has collapsed: the canvas-pointer-router (§4.2.1) classifies all pointerdowns from the always-on aboveView via `src/shared/hit-test.ts`, and every interactive surface that used to live in `bgView` or in a per-page `chromeView` WCV has moved into aboveView's React tree as `CanvasItemPopup` (`data-overlay-ui` so the router yields to them structurally). The per-page `chromeView` WCV and its `chrome-header` preload + renderer were retired wholesale; the chrome-action IPCs now use `canvas-navigate-page` (addressed by `pageId`) in place of the old sender-based `chrome-*` channels.

The `frameFocus` runtime field that ADR 0001 introduced no longer exists: keyboard target is derived from selection via `shouldFocusSelectedPage` (a pure predicate), and the gate stays open in canvas mode regardless. Pointer events that should reach a focused page are forwarded by main via `sendInputEvent` — see `src/main/runtime/page-input-forwarding.ts`.

### 4.2.1 Canvas pointer router (Phase 2 substrate)

A single window-level pointerdown listener inside `aboveView` (`src/renderer/above-view/useCanvasPointerRouter.ts`) runs the shared `hitTest()` against the current layout snapshot and dispatches a typed `CanvasPointerAction` (`src/shared/canvas-pointer-actions.ts`). The hit-test priority table — resize-handle > anchor > body > background — is encoded once and tested in isolation (`tests/unit/canvas-pointer-actions.test.ts`). The former `chrome` layer was retired with the chrome-header slot model ([ADR 0028](./adr/0028-retire-chrome-header-slot-model.md)); overlay UI anchors to the body rect and yields structurally via `data-overlay-ui`.

The router consumes the full action set (`FULL_ROUTER_CONSUME`): `enter-frame-focus`, `begin-entity-drag`, `begin-group-drag`, `begin-resize`, `begin-edge-drag`, `toggle-select`, `background-click`, `begin-marquee`, `begin-pan`. The remaining viewport helper is limited to wheel zoom/pan and middle-button pan.

Marquee selection is direction-independent: the default mode selects every item whose bounds intersect the marquee. Holding Cmd/Ctrl switches to full containment and lets the gesture begin through an item body; that origin item is excluded from the marquee. A stationary Cmd/Ctrl press still follows the normal additive-click selection path. Marquee resolution is group-aware in every mode: a fully enclosed group resolves to the group, while a partially crossed group is transparent and exposes its overlapping children.

Group movement uses the entity-drag IPC/session rather than a parallel gesture
channel. An unselected group's interior routes as canvas background (click to
clear, drag to marquee); its border and title remain direct drag handles. Once
selected, the interior moves the group subtree. During entity/group drags, the
renderer evaluates the pointer against the groups' pre-drag bounds, paints the
innermost target outline, and commits membership in the drag's existing undo
transaction. Releasing outside removes prior membership. Command-drag
suppresses group/page drop binding and clears its target feedback live;
Option-drag also suppresses group targeting while copying.

A sibling pure mapper, `routePointerDoubleClick`, classifies double-clicks; the router installs a window-level `dblclick` capture listener and dispatches `enter-shape-edit` / `enter-group` / `request-text-edit` (and yields `enter-group-rename` to the GroupRenameLabel's own DOM `onDoubleClick`). The text/shape branches use the `canvas-request-entity-edit` IPC channels, which select the entity in main and signal bgView to focus its inline editor.

**Click-on-solo-selected → edit (issue #49).** A click on a solo-selected text/sticky/shape body — or an editable file body — with no modifier and no active inline edit emits `begin-entity-press` rather than `begin-entity-drag`. The router defers resolution: a stationary release fires `canvas-request-entity-edit` (the same IPC the dblclick fast-path uses), and a pointermove that crosses the existing entity-press drag threshold falls through to a normal entity drag with `preserveSelection: true`. File entities qualify when the resolved renderer plugin declares `editable: true` in its registry claim — the value is broadcast as `rendererEditable` on the file scene entity and surfaced in the `entity-body` hit payload. Markdown / video opt in; image / component placeholders fall through to drag, and dblclick on those kinds is also a clean noop rather than entering `editing-entity` state with no editor on screen. Group, drawing, and page keep their kind-specific routes; group rename remains driven by `GroupRenameLabel`'s own `onDoubleClick`. Press-pending state lives as a hook-local ref inside `useCanvasPointerRouter`'s `runEntityPress`; no new `InteractionMode` is introduced (per §5.6 — payload changes suffice).

**Router responsibilities when canvas mode is active:**
1. Capture pointer events at the window boundary inside `aboveView`.
2. Hit-test against the current canvas scene via the shared pure `hitTest()` in `src/shared/hit-test.ts`, fed by the broadcast `layout-update` snapshot.
3. Classify the gesture (pan vs marquee vs entity drag vs resize vs edge drag vs edit) into a typed `CanvasPointerAction`.
4. Dispatch to a per-action handler that owns the gesture's pointer lifecycle (`runEntityDrag`, `runResize`, `runEdgeDrag`, etc.) and calls the typed IPC for that gesture (`api.beginDragEntity`, `api.beginResize`, …).
5. The main-side IPC handlers (`src/main/ipc/register-canvas-drag-ipc.ts` and siblings) are the layer that calls `InteractionController.tryEnter`, `commitActive`, `cancelActive`. Invariant I3 is enforced on the main side, not by the renderer router.

**Router does not:**
- Render page content.
- Own entity data.
- Call `tryEnter` / `commit` / `cancel` directly — those run in main, reached via typed IPC.
- Decide focus (that's `FocusReconciler`).

### 4.3 `LAYER_STACK` and the layout pass

```ts
const LAYER_STACK: readonly LayerDescriptor[] = [
  { id: 'bgView',       kind: 'singleton' },
  { id: 'pages',        kind: 'group', order: 'creation-order' },
  { id: 'aboveView',    kind: 'singleton' },
  { id: 'leftSidebar',  kind: 'singleton' },
  { id: 'devtools',     kind: 'cluster' },   // background, contents, header, resize
  { id: 'toolbar',      kind: 'singleton' },
] as const
```

**The layout pass is the only place any of these mutate:**

```ts
function layoutAllViews() {
  if (consumeDirty('stack')) applyStack(LAYER_STACK, state)
  if (consumeDirty('visibility')) applyVisibility(state)
  if (consumeDirty('bounds')) applyBounds(state)
  if (consumeDirty('pages')) applyPageLayout(state)
}
```

`requestLayout()` schedules the pass on a 16 ms timer. Every other subsystem (IPC handlers, observers, interaction controller, undo) calls `markDirty('<surface>')` and returns. **No one calls `setBounds`, `setVisible`, `addChildView`, or `removeChildView` outside the layout pass.** This is the invariant that makes the `setTimeout(0)` undo crutches unnecessary.

### 4.4 `FocusReconciler` (main)

```ts
interface FocusReconciler {
  expectedFocus(s: AppState): FocusTarget
  reconcile(): void   // called at the end of layoutAllViews
}

type FocusTarget =
  | { kind: 'aboveView' }        // canvas-mode default (post-Phase-F)
  | { kind: 'bgView' }           // legacy / explicit; not used as default anymore
  | { kind: 'page', id: string } // when shouldFocusSelectedPage elects a page
  | { kind: 'toolbar' | 'sidebar' }
```

`reconcile()` compares the expected focus to the actual focused webContents and calls `focus()` once if they disagree. It runs **after** all bounds/visibility changes so focus lands on a view that's actually visible. Every subsystem that might need focus (page-create, page-delete, tab-switch, undo cross-tab) sets a focus intent in state and lets the reconciler carry it out.

**Why a reconciler:** WebContentsView steals focus on load (#42578), macOS window-level focus doesn't fire webContents `blur` (#22201), and refocus callbacks cause storms if done reactively. A single post-layout reconciliation avoids all three.

### 4.5 `DropOwner` (main)

```ts
interface DropOwner {
  currentOwner(s: AppState): DropTarget
  isDuplicate(dragId: string, payloadHash: string): boolean
}

type DropTarget =
  | { kind: 'canvas' }           // drop creates entity at canvas coords
  | { kind: 'entity', id: string } // drop targets a specific entity (image into frame, etc.)
  | { kind: 'sidebar' }          // drop into workspace tree
  | { kind: 'none' }
```

On app start, every WCV registers a `dragover/drop` handler that calls `preventDefault` and forwards the event to main. Main consults `DropOwner.currentOwner()` and routes the drop. Dedup is by `dragId` (stamped on `dragstart`) — never by payload hash + timeout.

**Why ownership:** Electron's drag/drop delivery across overlapping WCVs is ambiguous (see #2897, #18226). Declaring ownership per drag eliminates the ambiguity rather than masking it.

### 4.6 Renderer gesture lifecycle (per-action handlers)

> **Status:** The original spec proposed a single `useDragGesture(spec)` hook that every gesture in `aboveView` would attach to a ref. That shape did not survive contact with the code — only one consumer (`useCanvasViewportGestures`) was ever ported, `useAnnotationDrawingGestures` was structurally incompatible with the ref-attaching model, and the lone consumer was later removed. The hook itself was deleted in #140. What stuck is the router pattern below.

Canvas-mode gestures live as per-action handlers inside `useCanvasPointerRouter` — one `runX` function per `CanvasPointerAction` kind (`runEntityDrag`, `runEntityPress`, `runPageBodyPress`, `runGroupDrag`, `runResize`, `runMultiResize`, `runEdgeDrag`, `runBackgroundSelectionGesture`, `runPan`, `runForwardPointer`, `runPlacementGesture`, `runCommentGesture`). Each handler runs its own pointer lifecycle inline: capture the pointer, install window-level `pointermove` / `pointerup` / `pointercancel` / `blur` listeners, run a per-gesture threshold check, dispatch IPCs into main on begin / move / commit / cancel, clean up on exit.

**Why per-handler, not a single primitive:**
- Several handlers diverge in load-bearing ways: `runForwardPointer` deliberately omits the `blur → cancel` listener (focus reconciliation blurs aboveView mid-gesture); `runEntityPress` and `runPageBodyPress` suppress pre-threshold blur for the same reason; `runResize` calls `api.beginResize` *before* the first patch dispatch to keep the focus reconciler from cancelling the gesture. A primitive that hid these would either need every nuance as a flag, or would force per-call workarounds.
- The router's call sites are imperative (the window-level pointerdown handler dispatches synchronously), not React components holding refs. A hook-with-ref API is the wrong shape for the actual caller.

**Shared conventions every handler must follow:**
- Pointer events only. No `mouse*` in new code (invariant I8).
- `setPointerCapture` on the event target where possible (`capturePointer` helper in the router).
- Window `blur` → cancel, *unless* the handler has a documented reason to ignore it (forward-pointer, pre-threshold press).
- Dispatch only typed IPCs from the renderer; never reach into `InteractionController` from renderer code.

**Boilerplate consolidation is an open opportunity, not a blocker.** The threshold-check + listener-install + cleanup scaffold is ~30 lines duplicated across most handlers. A non-hook helper — e.g. `withDragLifecycle(startEvent, { threshold, onMove, onCommit, onCancel })` callable from inside `runX` — could absorb that boilerplate without changing who owns gesture identity. Whether to take that refactor is a sizing question, not an architectural one.

### 4.7 Bitmap compositor (pages below the active set)

> **Status (ADR 0001 + ADR 0002):** Optional. The original motivation — keeping the gate always-on without breaking native page input — is supplanted by click-to-enter focus and (per [ADR 0002](./adr/0002-canvas-anchored-overlay-ui.md)) by moving canvas-anchored overlay UI into aboveView's React tree so the gate flip can't orphan it. The compositor is now a future memory/CPU optimisation if N-live-frame regresses, not a load-bearing input-authority requirement.

Inactive pages (not selected, not scroll-peer of selected, not loading, no DevTools) render via offscreen `BrowserWindow` with `offscreen: true` at low frame rate. Their bitmaps are drawn as React-rendered `<canvas>` elements inside `bgView`.

Full staging plan lives in `docs/offscreen-rendering-research.md`. The interaction-layer contract this spec establishes:

- Inactive pages appear to the input system as regions within `bgView`'s DOM.
- Clicking an inactive page triggers a **promote-to-live** transition: the offscreen BrowserWindow is destroyed, a new `pageView` WCV is created, scroll state is restored, the bitmap fades out.
- Promotions happen inside the layout pass, not in the click handler.

---

## 5. Heuristics (for future decisions)

### 5.1 Should this be its own WCV?

Add a WCV only if **at least one** is true:

1. It contains content you don't control (a web page, DevTools).
2. It has a genuinely different lifecycle (long-lived, rarely reloaded, different hot-reload needs).
3. Its crash is recoverable without the rest of the app.
4. Its z-order needs differ from other overlays in a way single-DOM composition can't express.

If none apply, it belongs in an existing WCV's React tree.

### 5.2 Should this state live in main or renderer?

**Main** if:
- More than one renderer needs to observe or mutate it.
- It persists across renderer reloads.
- An agent/CLI/test should be able to read or write it.
- It participates in undo.

**Renderer** if:
- It's pure visual ephemera (hover preview, dropdown-open state, scroll position within a panel).
- No other surface cares.

### 5.3 Should this go through the `InteractionController`?

Any interaction that:
- Can conflict with another interaction, or
- Has a visual drag preview, or
- Needs to be cancelable by blur/escape/undo

…goes through the controller. Momentary clicks (single select, button press) don't need it.

### 5.4 Should I add a `setTimeout(0)` here?

**No.** If you feel the urge, it means you're mutating view state during event dispatch. Mark dirty and return. If that doesn't work, the subsystem you're calling needs to move its mutation into the layout pass.

### 5.5 Which event family should I use?

Pointer events. Always. No new `mouse*` code.

### 5.6 Should I add a new interaction mode?

First check whether your behavior is an existing mode with a new payload. Modes are expensive — each one is a transition edge in the state machine, a visual in `aboveView`, a cancel path, and a test. Payloads are cheap.

---

## 6. Load-bearing invariants

These are the invariants that, if broken, produce the classes of bugs this refactor exists to eliminate. Any PR that violates one needs explicit discussion.

| # | Invariant | If broken |
|---|---|---|
| I1 | View-stack/visibility/bounds mutations only inside `layoutAllViews` | Event-routing breaks, spurious focus events, undo-during-drag corruption |
| I2 | One active `InteractionController` token at a time | Concurrent drags, orphan state, impossible undo steps |
| I3 | Every `tryEnter` pairs with `commit`, `cancel(token)`, or a blessed `cancelActive(reason)` from an external interrupter (§4.1). Enforcement lives on the main side, in the IPC handlers (`src/main/ipc/register-canvas-drag-ipc.ts` and siblings) that wrap each gesture begin/end pair — not in the renderer router | Stuck gestures, orphan visuals |
| I4 | Focus is expressed as intent, applied by `FocusReconciler` | Focus storms, keyboard shortcuts silently broken |
| I5 | Drop ownership is declared per `dragId`, never dedup by payload hash | Duplicate drops, missed drops |
| I6 | `setBackgroundColor('#00000000')` set on every WCV before `addChildView` | White-flash during creation |
| I7 | (ADR 0001 + aboveView migration) `aboveView` is the always-on canvas-mode input authority and the canvas-mode keyboard owner. The gate no longer toggles on a `frameFocus` runtime field — that field was retired; keyboard target is derived from selection via `shouldFocusSelectedPage` and pointer/wheel events that should reach a focused page are forwarded from main via `sendInputEvent` (`src/main/runtime/page-input-forwarding.ts`). Per-layer pointerdown handlers in `bgView` are gone. `cursorOverlayWindow` remains mouse-inert (`setIgnoreMouseEvents(true)`) | Regression to the multi-overlay-coordination model and the #41 layer-arbitration bug class |
| I8' | (ADR 0002, amended by ADR 0028) Canvas-anchored overlay UI (popups) lives in aboveView's React tree, not in `bgView` layers and not in per-page WCVs. Components tag themselves `data-overlay-ui`; the router yields to them on capture-phase pointerdown via `isOverlayUiTarget`. Geometry comes from `useAnchoredPosition` against the body rect (the chrome-header slot model was retired — entity rect == body rect) | Overlay UI stops receiving clicks when the gate flips fully open |
| I8 | Pointer events only in renderer gesture code | Divergent behavior between capture/cleanup code |
| I9 | Canvas coord math imported from `src/shared/coords.ts` | Hit-test drift between main and renderer |
| I10 | Live pages only for active frames + scroll peers + loading + DevTools-attached | Memory/CPU regression, idle renderers |

---

## 7. Electron gotchas (the ones that shaped this)

Cross-reference for future contributors. Each is why a choice above exists.

1. **WebContentsView has no z-order API.** Only `addChildView(view, index)` and `removeChildView` + re-add. → We formalize this via `LAYER_STACK` applied in `layoutAllViews`. [#42061]
2. **Overlapping WCVs do not pass pointer events through.** CSS `pointer-events: none` inside a WCV does not leak to a WCV beneath. → We minimize overlap (three planes) and gate input via visibility. [#45027]
3. **`setIgnoreMouseEvents` is BrowserWindow-only.** No per-view version. `forward: true` is macOS/Windows only, has reload quirks on Windows, and mousemove quirks on macOS. → We depend on it in exactly one place: `cursorOverlayWindow`, a child `BrowserWindow` sibling of `win`, used with `forward: false` (paint-only, no hover needed) to host agent-presence cursors. Its bounds are synced inside `layoutAllViews()` from `win.getContentBounds()` + `contentTopInset` plus listeners on `move` / `resize` / `focus` / `blur` / `enter-full-screen` / `leave-full-screen` / `display-metrics-changed`. The overlay stays hidden while `win` is unfocused because showing an OS-level child window can raise the app on macOS. A WCV cannot substitute because it would block native frame input. [#23863, #16777, #26718, #15376, #35030, #30808, #33281]
4. **WCVs default to opaque white background until load completes.** → Mandatory `setBackgroundColor('#00000000')` on creation. [#47351, #44914]
5. **WCVs auto-focus on navigation by default.** → Every WCV sets `webPreferences.focusOnNavigation: false`; `FocusReconciler` explicitly applies the expected focus post-layout when the main window is active. [#42578, #42922]
6. **Focus/blur on webContents don't fire for window-level focus changes on macOS.** → Expected-focus model instead of reactive focus chains. [#22201]
7. **Mutating the view stack during event dispatch loses events and can crash.** → Invariant I1. [#42131, #47247 + empirical observation in our undo path]
8. **`setAutoResize` doesn't exist on WebContentsView.** → Our layout pass explicitly resizes; not a regression. [#43802]
9. **Drag/drop events across overlapping views are ambiguous.** → `DropOwner` per `dragId`, never payload-hash dedup. [#2897, #18226, #7118]
10. **`sendInputEvent` requires the containing BrowserWindow focused.** → Not part of the default input path; reserved for offscreen-bitmap-click-through if/when needed.
11. **Native `setBorderRadius` exists on `View`.** → Use it for frame corners instead of compositing an extra overlay.
12. **`useSharedTexture` (Electron 33+) exists but needs a native addon.** → Not required for this spec; documented as the upgrade path for the compositor when IPC bandwidth becomes a bottleneck.

---

## 8. File layout (target)

```
src/main/
  runtime/
    interaction-controller.ts       # the state machine
    focus-reconciler.ts             # expected-focus resolution
    drop-owner.ts                   # drag-id keyed drop routing
    layout-engine.ts                # the single layout pass
    layer-stack.ts                  # LAYER_STACK descriptor + applyStack
    overlay-policy.ts               # OVERLAY_INPUT_POLICY table
    page-compositor.ts              # live vs bitmap tier management
    view-factory.ts                 # WCV creation with invariants (bg color, etc.)
  ipc/
    interaction-ipc.ts              # tryEnter/update/commit/cancel IPC
    input-gate-ipc.ts               # gate events from aboveView

src/shared/
  coords.ts                         # screen ↔ canvas math (SINGLE source)
  interaction-types.ts              # InteractionMode, GestureContext, tokens
  drag-ids.ts                       # dragId generator + types

src/renderer/
  above-view/                       # merged: interaction + comment + floating + annotation
    App.tsx
    useCanvasPointerRouter.ts       # window-level pointerdown router + per-action handlers
    MarqueeLayer.tsx
    DragPreviewLayer.tsx
    CommentsLayer.tsx
    AnnotationsLayer.tsx
    FloatingUiLayer.tsx
  bg-view/                          # canvas-bg, extended with:
    PageBitmapLayer.tsx             # bitmap compositor for inactive pages
    (...existing canvas chrome)
```

Retained as separate bundles: `toolbar/`, `left-sidebar/`, `devtools-*`. Retired: `interaction-overlay/`, `annotation-overlay/`, `floating-ui/` (merged into `above-view/`).

---

## 9. Testing strategy

### Unit

- `InteractionController` — full state-machine coverage, token lifecycle, concurrent `tryEnter` refusal.
- `FocusReconciler.expectedFocus` — pure function of state, no Electron.
- `DropOwner.currentOwner` — pure function of state.
- `coords.ts` — round-trip canvas↔screen, zoom/pan extremes.
- `layer-stack.applyStack` — deterministic output from state (no view list needed; work on descriptor IDs).

### Integration (in-process)

The Electron smoke layer this section originally specified is retired ([ADR 0024](./adr/0024-in-process-integration-testing.md)) — it exercised a test-only HTTP facade and was gated nowhere. What remains covered, and where:

- Gesture state-machine behavior (begin/commit/cancel, concurrent refusal, stale tokens) — unit, against `InteractionController`'s public API.
- Cross-module batch/undo integrity (a refused gesture-begin must not corrupt the undo batch, I3) — `tests/integration/interaction-batch.test.ts`, driving the same call sequence as the IPC handlers.
- Focus landing, gate visibility, and real drop routing depend on live views and are **intentionally uncovered** below the boot suite — the reconciler and drop-owner logic are unit-tested as pure functions of state; the wiring is validated by dogfooding.

### Agent / scenario

- Multi-frame workspace: marquee across frames, entity drag across frames, edge drag.
- Focus session: enter, work in a working tool, exit via Escape — focus toolbar tracks selection and gesture idle.

---

## 10. Glossary

| Term | Meaning |
|---|---|
| **Plane** | One of the three stacking regions: below-pages (`bgView`), pages, above-pages (`aboveView`). |
| **Input gate** | The behavior of `aboveView` toggling input capture via `setVisible`. Not a separate WCV. |
| **Gesture** | A pointer-initiated interaction with begin/update/commit/cancel phases. |
| **Mode** | The current `InteractionController` state. At most one non-idle mode at a time. |
| **Token** | Opaque handle returned by `tryEnter`, consumed by `commit`/`cancel`. Prevents orphan state. |
| **Live frame** | A frame rendered as a `pageView` WCV. |
| **Bitmap frame** | An inactive frame rendered offscreen and composited into `bgView` as pixels. |
| **Expected focus** | The `FocusTarget` a state implies; the reconciler enforces it. |
| **Drop owner** | The single WCV authorized to consume a given drag (keyed by `dragId`). |
| **Layout pass** | `layoutAllViews()`. The only place view-stack, visibility, and bounds change. |
| **Dirty flag** | A deferred-work marker consumed by the next layout pass. |
