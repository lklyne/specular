# Step 2 — FocusedNoteLayer ✅

The focused note now renders as a screen-fixed, fullscreen card: centered column (max 720px), full height under the flush-bar slot, opaque backdrop over the canvas, enter FLIP morph on the shared camera spring.

## Landed
- `FocusedNoteLayer.tsx` (new) — fixed `data-overlay-ui` layer outside the camera transform; card wraps `RendererSwitch` with auto-edit driven by `editingEntityId`; long notes scroll natively.
- FileBodyLayer suppresses the focused note's ordinary card; selection outlines/resize handles suppressed for the focused entity (`suppressPageId` generalized to `suppressFocusedId`).
- Main-side: other pages' WebContentsViews hide during a note session unless the eye is on (mirrors page-fill Amendment 2 semantics).
- Wheel is fully swallowed by the layer: card wheel scrolls the editor, gutter wheel no-ops, Cmd+wheel can't zoom-and-exit.

## Deviations
- **No exit morph.** At exit the camera restore is already tweening, so a frozen size tween fights the live rect (the documented popup trap). The layer hard-unmounts; the real card reappears near-fullscreen and the camera restore carries it out. Revisit only if manual testing says it reads badly.
- **Backdrop owns click-to-exit.** The pointer router bails on overlay UI before its focus-exit branch, so the backdrop calls `restoreFocusCamera()` itself, with a 4px drag guard so text selection can't accidentally exit.
- Layer spans from y=0 (behind the flush bar, z 10 vs bar z 20) to close a Cmd+wheel hole in the top strip.
- Overlay height uses `window.innerHeight` directly — the `canvasOrigin.y` subtraction used by popup culling double-counts the toolbar and would leave a 44px dead strip.

Verified: typecheck ✅, 1198 unit tests ✅.
