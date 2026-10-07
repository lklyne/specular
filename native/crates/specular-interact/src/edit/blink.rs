//! The caret's blink. It runs on the session's clock from the last time the
//! caret moved or the text changed, so the caret is solid while typing and
//! blinks once the keys stop.

use std::ops::Range;

use crate::App;

/// How long the caret shows, and then hides, while nothing is typed.
const HALF_MS: u64 = 500;

/// What the blink watches: the caret, the anchor, the text's length and the
/// composition. It restarts when any of them changes.
pub(crate) type CaretState = Option<(usize, usize, usize, Option<Range<usize>>)>;

/// The state of the edit's caret, to compare across an event with
/// [`restart_blink`].
pub(crate) fn caret_state(app: &App) -> CaretState {
    let edit = app.session.editing.as_ref()?;
    Some((
        edit.caret,
        edit.anchor,
        edit.text.len(),
        edit.composition.clone(),
    ))
}

/// Restarts the blink if the event just handled moved the caret or changed
/// the text.
pub(crate) fn restart_blink(app: &mut App, before: &CaretState) {
    if caret_state(app) != *before
        && let Some(edit) = &mut app.session.editing
    {
        edit.active_ms = app.session.now_ms;
    }
}

impl App {
    /// Whether the caret is in the shown half of its blink. It shows for
    /// half a second from the last time it moved or the text changed, then
    /// hides and shows by turns on [`Session::now_ms`](crate::Session::now_ms).
    /// `false` when no text is being edited.
    pub fn caret_visible(&self) -> bool {
        (self.session.editing.as_ref()).is_some_and(|edit| {
            let idle = self.session.now_ms.saturating_sub(edit.active_ms);
            (idle / HALF_MS).is_multiple_of(2)
        })
    }
}
