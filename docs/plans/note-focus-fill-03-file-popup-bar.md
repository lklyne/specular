# Step 3 — FilePopup focus bar ✅

FilePopup becomes the flush viewport-top focus bar during a note session, mirroring PagePopup's session binding and reusing the existing FLIP morph.

## Landed
- FilePopup binds to the focused file from `focusPresentation.target` regardless of selection; skips the show-delay; `placement="viewport-top"` + flush full-width frame.
- Bar contents while focused: rename label, markdown formatting buttons, eye toggle (show/hide context), X → exit. No arrange/annotate/fit-fill-device tabs.
- Eye-on makes the FocusedNoteLayer backdrop transparent so the canvas shows through around the card.
- `focusExempt` in the popup mutex table became a predicate keyed on the session's target kind — exactly one bar is exempt per session type.

## Deviations
- No App-side api plumbing was needed (popup table already passes the full API).
- `focusExempt` predicate (not boolean) — prevents FilePopup leaking past the tool mutex during a *page* session.

## Watch during manual smoke
1. **Eye-on is look-don't-touch** — revealed context is visible but clicks on it exit focus (backdrop still owns the pointer). Flag if you want real interaction there.
2. **Bar FLIP from keyboard-triggered focus** — with no previously-rendered file popup the bar may snap in instead of sliding (no baseline rect). Compare popup-button entry vs other entry paths.
3. **Formatting buttons** may be disabled for a frame while the editor registers.

Verified: typecheck ✅, 1198 unit tests ✅.
