# Note focus fill — build plan

Fullscreen "fill"-style focus for markdown notes, reusing the page focus-session machinery (ADR 0021).

## Steps
1. **Main foundation** — widen FocusSession to a page/file target union; FocusPresentationData keeps `pageId` (null for note sessions) + new `target`; `fillFocus` capability on the renderer registry (markdown only); focusSelection begins a note session (mode 'fill'), centers camera, auto-enters editing; session exits commit the edit.
2. **FocusedNoteLayer** — focused card renders in a fixed, data-overlay-ui layer (outside the camera transform): centered column, max-width 720, top inset TOOLBAR_HEIGHT; FLIP morph in/out with the shared camera spring easing; other content hidden per showsContext; wheel scrolls the editor.
3. **FilePopup focus bar** — session-bound, pinned flush to viewport top (same FLIP as the page focus bar), formatting + rename + eye + X exit; no fit/fill/device tabs.
4. **Final gate** — typecheck + unit + integration, then manual smoke by Lyle.

Per-step gate: typecheck + unit only. A new canvas note will be added per completed step with any deviations.

# Note focus fill — build plan

Fullscreen "fill"-style focus for markdown notes, reusing the page focus-session machinery (ADR 0021).

## Steps
1. **Main foundation** — widen FocusSession to a page/file target union; FocusPresentationData keeps `pageId` (null for note sessions) + new `target`; `fillFocus` capability on the renderer registry (markdown only); focusSelection begins a note session (mode 'fill'), centers camera, auto-enters editing; session exits commit the edit.
2. **FocusedNoteLayer** — focused card renders in a fixed, data-overlay-ui layer (outside the camera transform): centered column, max-width 720, top inset TOOLBAR_HEIGHT; FLIP morph in/out with the shared camera spring easing; other content hidden per showsContext; wheel scrolls the editor.
3. **FilePopup focus bar** — session-bound, pinned flush to viewport top (same FLIP as the page focus bar), formatting + rename + eye + X exit; no fit/fill/device tabs.
4. **Final gate** — typecheck + unit + integration, then manual smoke by Lyle.

Per-step gate: typecheck + unit only. A new canvas note will be added per completed step with any deviations.

# Note focus fill — build plan

Fullscreen "fill"-style focus for markdown notes, reusing the page focus-session machinery (ADR 0021).

## Steps
1. **Main foundation** — widen FocusSession to a page/file target union; FocusPresentationData keeps `pageId` (null for note sessions) + new `target`; `fillFocus` capability on the renderer registry (markdown only); focusSelection begins a note session (mode 'fill'), centers camera, auto-enters editing; session exits commit the edit.
2. **FocusedNoteLayer** — focused card renders in a fixed, data-overlay-ui layer (outside the camera transform): centered column, max-width 720, top inset TOOLBAR_HEIGHT; FLIP morph in/out with the shared camera spring easing; other content hidden per showsContext; wheel scrolls the editor.
3. **FilePopup focus bar** — session-bound, pinned flush to viewport top (same FLIP as the page focus bar), formatting + rename + eye + X exit; no fit/fill/device tabs.
4. **Final gate** — typecheck + unit + integration, then manual smoke by Lyle.

Per-step gate: typecheck + unit only. A new canvas note will be added per completed step with any deviations.
