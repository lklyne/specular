# ADR 0002 — Canvas-anchored overlay UI in aboveView

**Status:** Accepted — Steps 1–7 landed 2026-05-05; Step 8 (demolition) partial; smoke gate green as of 2026-05-07. **§1 (Shape B: entity rect includes its chrome) retired 2026-07-07 — see [ADR 0028](./0028-retire-chrome-header-slot-model.md).** The `CanvasItemChrome` component was never built; overlay UI ships as `CanvasItemPopup` anchored to the body rect, so the chrome-header slot became dead geometry (and caused #312). Entity rect == body rect now. **Extended by the aboveView interactive-layer migration (2026-05-06):** entity bodies, edges, group bounds, selection outlines / resize handles, focus ring, and agent halo now also render in aboveView; bgView is reduced to grid + camera + frame chrome; aboveView is the canvas-mode keyboard owner. See `docs/plans/aboveview-interactive-layer.md` and `docs/plans/aboveview-migration-journal.md` for the phased rollout (A → B → B′ → C → D → F).
**Date:** 2026-05-05
**Supersedes premise of:** the per-page `chromeView` `WebContentsView` created in `src/main/runtime/page-factory.ts`, and the per-bgView-layer interactive surfaces (group rename label, file chrome buttons, inline edit triggers).
**Builds on:** [ADR 0001 — click-to-enter frame focus](./0001-click-to-enter-frame-focus.md).
**Implementation log:** [`docs/divergence-input-authority.md`](../divergence-input-authority.md) — running status table.

## Context

ADR 0001 made `aboveView` the single input authority in canvas mode and routed all gestures through one window-level pointer router (`src/renderer/above-view/useCanvasPointerRouter.ts`). The architectural deepening landed on branch `refactor/input-authority-frame-focus` (see [`docs/divergence-input-authority.md`](../divergence-input-authority.md)), but the **gate-predicate flip is blocked** because several interactive surfaces still live in `bgView` (or in their own per-page WCV) and would stop receiving clicks the moment `aboveView` covers the canvas region:

- `chromeView` — per-page URL bar, nav buttons, action menus.
- `FileChromeLayer` buttons — file entity chrome.
- Group rename label — sits above group bounds.
- Inline text edit trigger — reached via dblclick today.

The original divergence doc treated each as its own migration. In practice they all want the same thing: **persistent or selection-driven UI rendered above a canvas entity, clickable, that survives the gate flip**.

## Decision

A single architecture for all canvas-anchored overlay UI:

### 1. Entity rect includes its chrome (Shape B)

The entity's bounding rect is one layout unit: body + chrome stacked vertically (or whichever placement the kind declares). Chrome's geometry is a runtime-derived sub-rect of the entity rect, **not** a persisted field — the `.canvas` schema stays unchanged.

- Resize handles attach to the body sub-rect.
- Edge anchors attach to the body sub-rect.
- Pan / zoom / drag / resize carry chrome automatically because there is nothing to offset.
- The "leave room for chrome above me" math currently scattered across gesture systems collapses.

A pure module `src/shared/entity-chrome-slots.ts` exposes per-kind functions: given an entity rect, return the chrome slot rect(s).

### 2. Overlay UI lives in aboveView's React tree

All persistent and selection-driven UI renders inside aboveView, anchored to entity rects via the layout broadcast aboveView already receives. Components are tagged `data-overlay-ui` so the existing pointer router yields to them (`useCanvasPointerRouter.ts` already calls `isOverlayUiTarget` on capture-phase pointerdown).

Two named components, one shared positioning hook:

| Name | Visibility | Examples |
|---|---|---|
| `CanvasItemChrome` | Always while entity exists | Frame URL bar, file buttons, group label |
| `CanvasItemPopup` | Selection-state-driven | Selection-contextual menus, action overflows |
| `useAnchoredPosition(entityId, slot)` | — | Pure hook, returns screen coords from layout broadcast |

Both follow the compound pattern (`CanvasItemChrome.Root / .DragTrigger / .Title / .Actions / .Button`) — same shape as the existing `EntityChrome` compound endorsed in `CLAUDE.md`. The `EntityChrome` compound moves to a neutral location so aboveView can import it.

### 3. Priority resolves structurally, not by table

The 5-layer priority table from ADR 0001 stays for *geometric* hits (resize handle, anchor, frame body, entity body, background). It does **not** include chrome anymore: chrome is plain React DOM in aboveView with `onClick` handlers. The router, on capture-phase pointerdown, sees `event.target` is inside `[data-overlay-ui]` and yields. The DOM event then runs through normal bubble-phase handlers.

This means "chrome wins over anchors" (the #41 fix) is structural — the DOM element catches the click before the router's hit-test runs at all. No new `HitPayload` kind for chrome is needed.

### 4. Selection-driven popups, not right-click

`CanvasItemPopup` visibility is a function of selection state, not a right-click gesture. Right-click context menus are out of scope for this change and will be a follow-up if needed.

### 5. UI copy voice

All chrome buttons, labels, menu items, dialog buttons use **sentence case** (capitalize first word only): "Reveal codebase in finder", not "Reveal Codebase in Finder". The lowercase-gerund convention applies only to cursor labels, status bar text, and live captions.

## Consequences

**Replaces:**
- The per-page `chromeView` `WebContentsView` created in `page-factory.ts` and its preload bridge — retired entirely.
- Per-bgView-layer `onMouseDown` handlers and the props that drilled them through canvas-bg layers — deleted in the same change as the gate flip (Phase 3 demolition).

**Enables:**
- Gate-predicate flip per ADR 0001 §"Decision" lands as a one-liner in the same PR.
- One uniform pattern for any future canvas-anchored UI (drawing toolbars, comment threads, AI agent badges, etc.).
- ESLint `no-mouse-events` rule promotes from per-file warning to project-wide error.

**Costs:**
- aboveView gains per-page chrome state subscriptions (URL, can-go-back, can-go-forward, is-loading, title) on the existing `layout-update` channel or a new sibling channel. Cost is real but bounded.
- `CanvasItemChrome` for frames re-implements URL bar / nav button UI that previously lived in the per-page chromeView preload. Existing IPC actions (`canvas-frame-chrome-action` family) port over.
- During the migration the entity rect's interpretation changes (now includes chrome). Per-kind layout code that assumed "rect = body" needs an audit.

**Out of scope:**
- Right-click context menus (selection-driven popups only for this PR).
- Persisted chrome in the `.canvas` schema (chrome stays a render-time concern).
- Bitmap compositor / Phase 6 from `interaction-layer.md` §4.7 (already out of scope per ADR 0001).

## Landing as a single PR

The change is sweeping but coherent; it ships as one PR rather than incrementally.

1. `src/shared/entity-chrome-slots.ts` — pure per-kind chrome slot geometry + tests.
2. `src/renderer/above-view/useAnchoredPosition.ts` — positioning hook.
3. `CanvasItemChrome` + `CanvasItemPopup` compound components + the moved `EntityChrome` compound primitives.
4. `<FrameChrome>`, `<FileChrome>`, `<GroupRenameLabel>` consumers in aboveView.
5. Per-page chrome state IPC plumbing to aboveView; retire per-page `chromeView` WCV and its preload.
6. `routePointerDoubleClick` extension to `canvas-pointer-actions.ts`; dispatch `enter-shape-edit | enter-group | enter-group-rename | request-text-edit`. New `request-text-edit` IPC.
7. Gate predicate flip per ADR 0001 plan Step 1.
8. Phase 3 demolition: delete the old bgView gesture hooks; remove bgView/page-content canvas gesture authority; promote `no-mouse-events` to error after remaining native/editing handlers are audited.
9. Docs: amend `interaction-layer.md` §4.2/§4.7/§6; close out `divergence-input-authority.md` (this ADR replaces its "Recommended next sequence").

## Implementation status (2026-05-05)

| Step | State | Notes |
|---|---|---|
| 1. `entity-chrome-slots.ts` | ✅ landed → 🗑️ deleted | + `tests/unit/entity-chrome-slots.test.ts`; subsequently deleted by ADR 0028 (retire-chrome-header-slot-model, 2026-07-07) |
| 2. `useAnchoredPosition` | ✅ landed | + `tests/unit/use-anchored-position.test.ts` |
| 3. `CanvasItemChrome` / `CanvasItemPopup` | ⚠️ partial | `CanvasItemPopup` landed (see ADR 0008). `CanvasItemChrome` was **never built** — overlay UI shipped as `CanvasItemPopup` anchored to the body rect; see ADR header note. `EntityChrome.tsx` was a stepping-stone compound but ships without the chrome-header band. |
| 4. `<FrameChrome>`, `<FileChrome>`, `<GroupRenameLabel>` consumers in aboveView | ✅ landed | `src/renderer/above-view/{FrameChrome,FileChrome,GroupRenameLabel}.tsx`; canvas-bg mounts removed |
| 5. Per-page chrome IPC + retire `chromeView` WCV | ✅ landed | Page type, layout-engine, navigation-sync, runtime-core, preferences all stripped of `chromeView`; `chrome-header` preload + renderer + Vite/Forge entries deleted; `register-page-chrome-ipc.ts` chrome-* handlers removed |
| 6. `routePointerDoubleClick` extension + IPC dispatcher | ✅ landed | `canvas-request-text-edit` / `canvas-request-shape-edit` IPCs added; `text-begin-edit` listener wired; `pendingTextEditId` auto-focuses the textarea in `TextBlockLayer` |
| 7. Gate predicate flip | ✅ landed | `shouldGateBeOpen` returns `true` in canvas mode with carve-outs for `frameFocus`, `editing-text`, and `inspect`/`annotate-comment` (without composer); `gate-predicate.test.ts` rewritten |
| 8. Phase 3 demolition | ✅ landed | bgView selection, resize, group-drag, edge-drag, and viewport left-button gesture hooks are removed or visual-only. Canvas gestures route through aboveView's pointer router; page-content no longer sends page select/group-drag canvas IPC. |
| 9. Docs cross-refs | ✅ landed | `interaction-layer.md` §4.2/§4.7/§6, `divergence-input-authority.md` status table, `architecture.md` + `CLAUDE.md` `chrome-header` references purged |

### Verification

- `pnpm typecheck` — clean.
- `pnpm test:unit` — 355/355 pass.
- `pnpm test:smoke` — 76/76 pass (after 2026-05-07 fix). Initial gate-end run reported 3 failures (`agent-canvas presence cleanup`, `cdp-proxy reuses stable proxy url`, `selection > group overlay non-interactive/multiSelected`); root-caused to test pollution, not the gate flip. The smoke config ran files in parallel against a single shared Electron HTTP server, so concurrent files raced on selection / cdp-proxy / presence state. Adding `fileParallelism: false` to `vitest.smoke.config.ts` makes file execution serial; all three failures cleared on both `aboveview-migration` and `main`. The hypothesised `getSelectionOverlayState.interactive` regression was a false lead.

### Open follow-ups

- Consider promoting `local/no-mouse-events` to `error` after auditing the remaining native/editing-only mouse handlers.
- Wireframe JSON-mode toggle restored 2026-05-07 — chrome and body share the aboveView WCV, so a `useState` Map in `App.tsx` with a setter threaded into `FileChromeOverlay` was sufficient (no IPC channel needed, contrary to the original removal note).

## Tests

- Unit: `entity-chrome-slots.test.ts`, dblclick branches in `canvas-pointer-actions.test.ts`, gate-predicate frame-focus branch (already landed).
- Smoke: #41 anchor-near-chrome regression; chrome nav buttons fire IPC; group rename works; file chrome buttons fire IPC; dblclick on text entity opens editor; Escape exits frame focus.
- Manual: pan/zoom/drag/resize tracking for `CanvasItemChrome` instances under stress.
