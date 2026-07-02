# Ponytail audit — whole-repo cut list (base 36eb937)

Run 1 of docs/architecture-audit.md. Ground truth: docs/audit/fallow-36eb937.txt.
Every finding verified by reading code (three parallel hunters: src/main, src/renderer,
shared/preload/packages/deps). Reports only; nothing applied. ADR-protected systems
(two-layer Yjs sync, renderer camera, plugin/tool registries, focus session,
select-first pages, stack order) verified untouched by every finding below.

## Cut list (ranked, biggest first)

- `delete` Debug-window dead weight: "Cursor motion (legacy)" section (CursorMotionSection + PlaygroundCanvas + ControlsPanel, ~486 lines) + its prefs/IPC plumbing (~60) + PresenceTimelinePanel (~340 — subscribes to `presence-timeline-append`, which nothing ever emits; renders an empty timeline forever). Verified: `cursor-motion-changed`/`cursor-spline-viz-changed` have zero non-debug listeners; `AgentCursorLayer` uses the `DEFAULT_CURSOR_MOTION` constant. Replacement: nothing. [src/renderer/debug/CursorMotionSection.tsx, src/renderer/debug/PresenceTimelinePanel.tsx, src/main/runtime/preferences.ts:300] (~885 lines)
- `delete` Wireframe fixture JSONs tracked since initial commit, referenced nowhere in src/tests/docs/scripts. Replacement: nothing. [example.wireframe.json, grain-gradient.wireframe.json] (~264 lines)
- `shrink` 12 hand-rolled pointer drag-session scaffolds (capturePointer → pointerId filter → 4× add/removeEventListener → cleanup → phantom-blur guard), 11 in the router + 1 in App.tsx — fallow's 269-line in-file dup cluster. Replacement: one `startPointerSession(event, {onMove,onUp,onCancel,listenBlur})` helper (+ tap-vs-drag threshold variant); must keep the phantom-blur guard as an option (load-bearing per focus-session). [src/renderer/above-view/useCanvasPointerRouter.ts:436,514,607,746,924,1085; src/renderer/above-view/App.tsx:998] (~200 lines)
- `shrink` Preload subscription closure copy-pasted 36× across 10 bridge files. Replacement: 6-line `on<T>(channel)` helper in src/preload/ipc-helpers.ts; each call becomes one line, still IPC-only. [src/preload/canvas-bg.ts:72, src/preload/toolbar.ts:26] (~110 lines)
- `shrink` Edge geometry (`getAnchorPoint`, `controlPointOffset`, `buildBezierPath`, `autoSides`, `CONTROL_POINT_MIN/MAX`) exists twice; EdgeLayer's copy only adds an `originY` param. Replacement: one module, `originY = 0` default. Bonus boundary fix: edge-drag-controller.ts is renderer-only (all callers in above-view) — move it out of shared/ and merge. [src/shared/edge-drag-controller.ts:184-341 vs src/renderer/above-view/EdgeLayer.tsx:34-130] (~100 lines)
- `shrink` Body-layer scaffold: `FileViewportLayer`/`ShapeViewportLayer`/`StickyViewportLayer`/`GroupViewportLayer` are 4 byte-identical 23-line components; the three `*Shell` divs share the absolute/`data-entity-id`/touchAction skeleton. Replacement: one `CanvasViewportLayer` + style-parameterized `EntityShell` (card internals genuinely differ — don't force further). [src/renderer/above-view/FileBodyLayer.tsx:29, ShapeBodyLayer.tsx:28, StickyBodyLayer.tsx:43, GroupBoundsLayer.tsx:20] (~100 lines)
- `shrink` Pen-family icons re-declare identical mask/body/shine/seam gradient defs + 8-color derivation per icon (can't be static SVG — live `ink` tint and `isDark` recolor are product behavior). Replacement: shared `<PenIconDefs prefix isDark>` + palette helper. [src/renderer/shared/CustomIcons.tsx:109-144 vs 284-323] (~100 lines)
- `delete` Presence-move forensic logging (`logPresenceMove`, `coordSourceForLog`, `formatCoord`, `formatTargetRect` incl. `suspect=` investigation markers) behind `PRESENCE_MOVE_LOGGING_ENABLED`. Replacement: ~10-line plain log or nothing. [src/main/presence-cursor.ts:262-361] (~90 lines)
- `yagni` Pure re-export facades beside their real modules — `surface-layout.ts` (49 lines, renames symbols) and `workspace-session.ts` (32); callers already split ~half-and-half between facade and direct imports. Replacement: direct imports. Caveat: entangled in the #141 suppressed circular-dep cluster — verify cycles don't worsen. [src/main/runtime/surface-layout.ts, src/main/runtime/workspace-session.ts] (~80 lines)
- `shrink` Field-by-field merge boilerplate: `patch.x === undefined ? existing?.x ?? null : patch.x` × ~15 fields × 2 functions (both fallow-CRITICAL). Replacement: `{...defaults, ...existing, ...defined(patch)}` with a strip-undefined helper. [src/main/presence-cursor.ts:404-458,495-536] (~75 lines)
- `delete` Resolved feature flags + dead branches: `DRAWING_FEATURE_ENABLED = true` (~10 branches incl. the whole `isCanvasEntityKindEnabled` plumbing), `PERFECT_FREEHAND_ENABLED = true`, `FOCUS_PRESENTATION_MENU_INSET = false`. Replacement: inline winners; featureFlags.ts keeps only the one live flag. [src/shared/featureFlags.ts:3-32, src/main/ui-state.ts:26-215] (~70 lines)
- `yagni` Custom cubic-bezier easing solver (`solveCubicBezierX` Newton-Raphson + bisection, `cubicBezier1D/Deriv`, `kind:'custom'` arms) — only producers are the legacy debug section deleted above. Replacement: presets only. [src/shared/cursor-motion.ts:81-128,317-339] (~55 lines)
- `shrink` PresenceLabelKey listed 3×: type union (types.ts:502), runtime `Set`s (presence-cursor.ts:94-122), 24-branch switch (agent-presence.ts:3-44). Replacement: one `as const` array, derived type, `Record<PresenceLabelKey, string | ((t) => string)>`. [src/shared/types.ts:502] (~45 lines)
- `shrink` 13 one-field IPC handlers (`set-page-preset` … annotation ops) all validate-one-field → call-one-fn. Replacement: channel→command table. [src/main/ipc/register-right-details-panel-ipc.ts:158-202,271-345] (~45 lines)
- `yagni` Dead props/params across renderer: `onSelect` on DrawingsLayer (never passed → hit-path never renders), `scaleWithZoom` (always false) + `beginResize` (never passed) on ResizeHandles, `align` on CenterAddressBar, `defaultSelected()` ignoring its arg, `onAutoFocusConsumed` on MarkdownEditor. Replacement: nothing. [src/renderer/above-view/DrawingsLayer.tsx:167, src/renderer/canvas-bg/ResizeHandles.tsx:9, src/renderer/toolbar/toolbarSections.tsx:380] (~41 lines)
- `yagni` presence-manager barrel facade re-exporting presence-cursor/presence-session (not even consistently honored — app-control-server bypasses it) + literal duplicated line at :133-134. Replacement: import real modules. [src/main/presence-manager.ts:29-80] (~40 lines)
- `stdlib` Hand-rolled requestId+once+timeout IPC correlation, twice, missing the per-page cleanup the existing generic helper has. Replacement: route through `sendPageIpc` (src/main/runtime/page-ipc.ts:35). [src/main/app-control-server.ts:387-450] (~35 lines)
- `shrink` Two clone groups inside WireframeNodeRenderer.tsx. Replacement: local extraction. [src/renderer/canvas-bg/wireframe/WireframeNodeRenderer.tsx] (~35 lines)
- `dup` `ComponentPropOverridePayload` + `ComponentTokenOverridePayload` + `forwardOverrideToPage` byte-identical in two IPC registrars. Replacement: extract once into src/main/ipc/. [register-annotation-inspection-ipc.ts:34-57 vs register-right-details-panel-ipc.ts:49-72] (~30 lines)
- `delete` `canvasGeometry.ts` (`unionScreenBounds`) — zero imports anywhere (fallow's unused file, confirmed). Replacement: nothing. [src/renderer/canvas-bg/canvasGeometry.ts] (~27 lines)
- `stdlib` 300ms debounced-file-write block copied 3×. Replacement: one shared hook. [MarkdownInlineRenderer.tsx:82, WireframeInlineRenderer.tsx:71, StickyBodyLayer.tsx:199] (~20 lines)
- `native` Two hand-rolled textarea autosize routines. Replacement: CSS `field-sizing: content; max-height: 120px` (Electron 40's Chromium has it). [src/renderer/above-view/useAnnotationDraftState.ts:42, src/renderer/right-details-panel/useElementCommentDraft.ts:22] (~19 lines)
- `dup` `selectionDebug` defined 3× in main while runtime-constants.ts:41 exports it (preload's 4th copy must stay — separate bundle). Replacement: import it. [src/main/runtime/runtime-core.ts:185, register-app-ipc.ts:15, register-page-chrome-ipc.ts:26] (~15 lines)
- `shrink` `toAcceleratorKey` 28-line switch. Replacement: 12-entry lookup table + `?? key.toUpperCase()`. [src/shared/bindings.ts:329-357] (~15 lines)
- `delete` `VERB_PRESENCE` sends label keys the allowlist silently nulls — config that does nothing. Replacement: collapse to keys that validate (or fix the allowlist — current values are fiction either way). [src/main/cli-presence.ts:8-30] (~12 lines)
- `yagni` `MutationContext` intentionally-empty interface threaded through every entity-kind handler ×6 kinds. Replacement: add the param when it first exists. [src/main/entities/contract.ts:38-50] (~10 lines)
- `yagni` `applyTaskLayout` generic task envelope with exactly one supported kind. Replacement: inline `'breakpoint_map'`. [src/main/workspace-layout-tasks.ts:290-383] (~10 lines)
- `dup` `currentlyFocusedKey` copy-pasted into test routes (the real content of fallow's "493-line clone", which is otherwise a false positive). Replacement: export from focus-reconciler-runtime, import. [src/main/routes/test.ts:97-106] (~10 lines)
- `delete` No-op `page-hover` IPC handler — guards then unconditional return; unhandled `ipcMain.on` already drops silently. Replacement: delete handler, keep comment. [src/main/ipc/register-page-chrome-ipc.ts:48-56] (~8 lines)
- `stdlib` `JSON.parse(JSON.stringify())` deep clone. Replacement: `structuredClone` (Node 22). [src/main/runtime/runtime-serialization.ts:56-64] (~4 lines)
- `stdlib` `@sentry/core` direct dep for one type-only import. Replacement: `Sentry.Event` from `@sentry/electron/main`. [src/main/sentry.ts:2] (-1 dep)
- `shrink` types.ts (2,278 lines) is ~97% type declarations — the "validator bloat" theory is false — but ~50 executable lines (`resolveSpacing`, `validateLayoutDirective`, …) belong in their own module so types.ts is type-only. Replacement: move to src/shared/layout-directive.ts; keep the hand-rolled validator (zod-for-one-validator would be over-engineering). [src/shared/types.ts:1466-1529] (0 lines, hygiene)

## Tradeoffs — owner decisions (2026-07-01)

- `three.js` — **DECIDED: KEEP.** The GPU particle trail is deliberate craft. Do not propose again.
- Debug window — **DECIDED: keep for local dev, stop shipping it.** New action item for the
  apply pass: gate the debug window out of packaged builds (`app.isPackaged`) — the menu item +
  Cmd+Shift+D shortcut (`src/main/app-menu.ts:175`), the `debug-window.ts` open path, and ideally
  the build entry (`vite.renderer.config.ts:27`). The hard-dead pieces in the cut list
  (PresenceTimelinePanel, legacy cursor-motion section) stay deletable — they're dead even for
  local dev (empty timeline forever; knobs nothing reads). The PresenceSection + PresencePlayground
  simulator (~1,010 lines) stays.
- MCP surface (~1,000 LOC): clean parity wrapper over the same HTTP API the CLI wraps. CLAUDE.md says agents are moving to the CLI — when that lands, this is the biggest single delete available. Product decision, not dead code today — **undecided, revisit after CLI migration**.
- App.tsx owns a second `pointerdown` gesture site (placement/comment tool) beside the router — consolidating honors the single-dispatch-site invariant but touches load-bearing routing. The `startPointerSession` extraction above works either way. **Undecided — candidate for run 3 (`deep-audit src/renderer`).**

## Out of scope (route to a bug pass)

- packages/vite `matchesAllow` glob is broken for its own README example (`'**/*.tsx'` never matches); package is also `"private": true` while its README says to install it. Correctness bug, not slop.

## Verified fine — do not cut

routes/test.ts (dev-gated smoke hooks; the 493-line fallow clone is a false positive) ·
app-control-server's 562-line function (it IS the shared router; split for readability, saves ~0) ·
entities/builtin + *-entity-state (ADR 0019: one mutation path, deliberate) ·
presence scoring/targeting (product code, used by routes) ·
url.ts / gesture-utils / structured-content / reorderable-dots (each earns its lines) ·
big preload files (guest-page isolated-world code — must live there) ·
cli-parser.ts (65 lines; parseArgs wouldn't be smaller) ·
deps ws / react-markdown / @codemirror/* / perfect-freehand / update-electron-app / @modelcontextprotocol/sdk / lucide-react / @base-ui/react / yjs (all right-sized) ·
eslint-rules, scripts/ (load-bearing).

## Net

net: **~2,550 lines, -1 dep** surgical (no behavior change) + prod-gating the debug window per the decision above.
Future: MCP delete (~1,000) after CLI migration. three.js is off the table.

## Status / next step

- [x] Run 0 — fallow ground truth (`fallow-36eb937.txt`)
- [x] Run 1 — this cut list; tradeoffs decided 2026-07-01
- [~] **Apply pass** (in progress, branch `architecture-audit`) — each commit verified with
      `pnpm typecheck && pnpm test:unit && pnpm test:smoke` (all green: 722 unit, 169 smoke).
      Applied so far:
  - `56b0c5d` — canvasGeometry + wireframe fixtures + page-hover handler deletes; feature-flag
    resolution (DRAWING/PERFECT_FREEHAND/FOCUS_PRESENTATION_MENU_INSET + isCanvasEntityKindEnabled
    + drawingEnabled prop); VERB_PRESENCE collapse; structuredClone; `@sentry/core` dep drop.
  - `e548d65` — presence-move forensic logging; dead props (onSelect, ResizeHandles
    beginResize/scaleWithZoom, CenterAddressBar align, MarkdownEditor onAutoFocusConsumed,
    defaultSelected); presence-manager duplicated derivePageId line.
  - `1e89866` — legacy cursor-motion debug section + PresenceTimelinePanel + presence-debug.ts
    deletes; debug window prod-gated (app.isPackaged).
  - `4aef9e3` — toAcceleratorKey lookup table; selectionDebug dedup (2 IPC copies);
    currentlyFocusedKey dedup.

  **Skipped (with rationale):**
  - `MutationContext` empty interface — documented ADR-0019 seam, on this file's own do-not-cut list.
  - Finding #22 (cubic-bezier easing solver) — the *kept* PresenceSection easing picker still
    produces `kind:'custom'` specs, so it is not dead; cursor-motion.ts left untouched.

  **Not yet applied (recommend one focused commit each; several are load-bearing — review before/after):**
  - Dedup extractions: pointer drag-session scaffolds (~200, interaction-layer §6 invariants),
    preload `on<T>` helper (~110), edge geometry merge + boundary move (~100), body-layer scaffold
    (~100), pen-icon defs (~100), presence-cursor field merge (~75, fallow-CRITICAL),
    PresenceLabelKey 3× (~45), 13 one-field IPC handlers (~45), WireframeNodeRenderer clones (~35),
    ComponentPropOverridePayload dup (~30), debounced-file-write hook (~20), selectionDebug/... done.
  - Facades: surface-layout / workspace-session (~80, #141 circular-dep caveat),
    presence-manager re-export blocks (~40, repoint 7 consumers).
  - stdlib: requestId+once+timeout IPC → sendPageIpc (~35).
  - native: two textarea autosize routines → CSS `field-sizing: content` (~19) — **needs visual
    verification in the running app** before applying.
  - yagni: applyTaskLayout generic envelope (~10). hygiene: move layout-directive out of types.ts.
- [ ] Run 2 — `/improve-codebase-architecture` (prepend the invariant preamble from
      `docs/architecture-audit.md`; point it at this file so it doesn't re-derive)
- [ ] Run 3 — `deep-audit` per seam, heaviest first (see docs/architecture-audit.md §3)
