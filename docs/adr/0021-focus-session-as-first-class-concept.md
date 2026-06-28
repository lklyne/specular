# ADR 0021 — Focus Session as a First-Class Concept

Status: Accepted

Date: 2026-06-28

## Context

[ADR 0020](./0020-delete-browser-mode-for-focus-selection.md) replaced Browser
mode with `Focus selection`: an ephemeral camera command that frames one page,
dims the rest, and remembers a return camera. It was built as a *presentation*
affordance — lock the camera, dim, read.

The product has since pulled focus toward a *working* surface: users want to
draw, annotate, and add items while a page is focused. That requirement exposed
the original implementation as undermodeled. "Focus" was never a concept in the
code — it was an emergent side-effect of state spread across subsystems:

- The session state itself was split across two modules: `focusPresentationOverride`
  (page + mode) in `runtime-context.ts` and `focusReturnCamera` (the return
  camera) in `viewport-control.ts`. One concept, two variables, no name.
- Seven main-side readers each re-derived "are we focused" from the bare
  `focusPresentationOverride` flag.
- The focus toolbar keyed off *selection*, not focus — so draw mode (which
  clears selection) silently collapsed it.
- "What ends a focus session" was implicit across ~half a dozen sites: every
  `setZoom`/`setPan`, the pointer router's dimmed-canvas click, the exit button,
  and the Escape binding. Each independently remembered to check the flag, so
  every new bug surfaced a new exit path.

This is the exact anti-pattern `interaction-layer.md` built `InteractionController`
to eliminate — "the controller holds the single truth; no one else derives it"
(§4.1). Focus presentation was the one cross-cutting state that never got that
treatment.

## Decision

Promote focus presentation to a **first-class session with a single owner**,
`src/main/runtime/focus-session.ts`. It is a sibling to `InteractionController`
and `FocusReconciler`, **not** a new `InteractionMode` — focus spans gestures,
it is not itself a gesture (per §5.6, modes are expensive; this is session
state, not a transition edge).

1. **Unify the state.** One `FocusSession` object owns `{ pageId, mode,
   returnCamera }`, captured when the session begins. The two old variables are
   deleted. A session always carries a return camera, so the presence of a
   session *is* the "is this restorable" test — `hasFocusReturnCamera()` is just
   `isFocusSessionActive()`. (A return-point-less session — e.g. one entered via
   reset-viewport — was considered and dropped: nothing creates one today, so
   the nullability was speculative. Add it back the day a producer needs it.)

2. **One writer, enumerated exits.** Every exit funnels through
   `endFocusSession(reason)`. The reasons are a closed set:
   - `dismiss` — the graceful, camera-restoring exit (X button, Escape,
     dimmed-canvas click all map here via `restoreFocusCamera`);
   - `camera-change` — the user moved the camera (`setZoom`/`setPan`);
   - `re-focus` — `focusSelection` re-entered on a target that isn't a single
     page.

   "What ends focus" is now one auditable list instead of a scavenger hunt.

3. **Tightening — focus survives working tools.** A `camera-change` no longer
   ends the session while a *working tool* (`isWorkingTool`: any placement tool,
   plus draw and comment) is active. You are annotating or placing, not leaving.
   The dim is likewise a resting affordance: a working tool lifts it. Both read
   the same `isWorkingTool` predicate so "am I working" has one definition.

4. **Consumers read the session, never re-derive.** Main readers call
   `focusSession()`. Renderers derive everything through one shared selector,
   `focusContext(layout)` in `src/shared/focus-context.ts` (pure, operates on
   the broadcast `LayoutUpdateData`, usable by both the above-view and canvas-bg
   bundles). The focus toolbar resolves its page from the session, independent
   of selection.

## Consequences

- "Focus session" is a named thing future product work can build on, rather than
  a flag five subsystems happen to agree on.
- Drawing, placing, and commenting no longer drop the focus toolbar or fight the
  camera, because nothing load-bearing reads selection or re-derives focus.
- Adding or changing an exit trigger means editing one function with a typed
  reason — the failure mode of "found another path that clears focus" is gone.
- The dim subsystem is now explicitly a *resting* affordance. Whether a working
  focus surface should dim at all is left as a product question; the seam is now
  in one place (`focusContext().dimsOtherPages`) if we revisit it.
- This is a pure structural consolidation of existing behavior plus the one
  documented tightening; it does not touch the gesture architecture
  (`InteractionController`, the pointer router, the layout pass), which held up
  fine.
