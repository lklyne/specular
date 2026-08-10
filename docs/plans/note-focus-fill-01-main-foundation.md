# Step 1 — Main foundation ✅

Focus session widened to a page/file target; markdown notes can now own a focus session.

## Landed
- `FocusSession.target: { kind: 'page' | 'file', id }` with `focusedPageId()` / `focusedFileId()` helpers — all page-specific consumers no-op for note sessions.
- `FocusPresentationData.pageId` is now nullable; new `target` field broadcast to renderers.
- `fillFocus` renderer capability (markdown only), broadcast as `rendererFillFocus` on scene entities.
- `focusSelection()` on a single markdown note: begins a `'fill'` session, fits the camera, auto-enters the editor.
- Every session exit (escape/click-out/camera-change/re-focus/delete) commits an in-flight note edit.

## Deviations
- Auto-edit lives in `focusSelection`, not the IPC layer — toolbar zoom paths also create note sessions and need the editor.
- `refocusActiveSession` also commits a pending note edit before repointing to a page (safety not in spec).
- Delete-path skips `syncInteractiveToFocus` — provably a no-op during file sessions.

## Carried to Step 2
- Layout engine no-ops for file sessions, so other pages' webviews stay visible during a note session — Step 2 hides them (mirrors "fill always hides other pages").

Verified: typecheck ✅, 1198 unit tests ✅.
