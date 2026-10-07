//! The pointer in the text being edited: a click places the caret, a double
//! click takes a word and a triple click a paragraph, and dragging from any
//! of them extends the selection by the same unit.

use std::ops::Range;

use glam::DVec2;

use super::segment;
use crate::{App, Gesture, Hit, PointerInput, hit};

/// What a selection drag grows by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Grapheme,
    Word,
    Paragraph,
}

impl Unit {
    const fn of(click_count: u8) -> Self {
        match click_count {
            0 | 1 => Self::Grapheme,
            2 => Self::Word,
            _ => Self::Paragraph,
        }
    }

    /// The whole unit at `offset`.
    fn at(self, text: &str, offset: usize) -> Range<usize> {
        match self {
            Self::Grapheme => offset..offset,
            Self::Word => segment::word_at(text, offset),
            Self::Paragraph => segment::paragraph_at(text, offset),
        }
    }
}

/// A press inside the text being edited, up to its release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSelectDrag {
    /// What the press selected, which the drag never lets go of.
    origin: Range<usize>,
    unit: Unit,
}

/// The offset in the edited text nearest the canvas point `world`.
fn offset_at(app: &App, world: DVec2) -> Option<usize> {
    let edit = app.session.editing.as_ref()?;
    let (frame, layout) = super::geometry(app, edit)?;
    Some(frame.offset_at(&layout, world))
}

/// Offers a left press to the text being edited. Returns the drag it starts
/// when the press is on that entity's body, and `None` when it is anywhere
/// else, which ends the edit.
pub(crate) fn press(
    app: &mut App,
    input: &PointerInput,
    click_count: u8,
) -> Option<TextSelectDrag> {
    let editing = app.session.editing.as_ref()?.entity.clone();
    match hit::hit_test(app, input.screen) {
        Hit::EntityBody { entity } if entity == editing => {}
        Hit::EntityBody { .. }
        | Hit::GroupLabel { .. }
        | Hit::Handle { .. }
        | Hit::Anchor { .. }
        | Hit::PageContent { .. }
        | Hit::GroupBorder { .. }
        | Hit::Edge { .. }
        | Hit::Empty => return None,
    }
    let world = app.session.camera.screen_to_world(input.screen).as_dvec2();
    let offset = offset_at(app, world)?;
    let edit = app.session.editing.as_mut()?;
    // A click takes the text out of the input method's hands as it stands.
    edit.composition = None;
    let unit = Unit::of(click_count);
    let origin = if input.modifiers.shift && unit == Unit::Grapheme {
        edit.move_to(offset, true);
        edit.anchor..edit.anchor
    } else {
        let taken = unit.at(&edit.text, offset);
        edit.select(taken.clone());
        taken
    };
    Some(TextSelectDrag { origin, unit })
}

/// The pointer moved with `drag` in flight: the selection runs from what
/// the press took to the unit under the pointer.
pub(crate) fn drag(app: &mut App, drag: &TextSelectDrag, world: DVec2) {
    let Some(offset) = offset_at(app, world) else {
        return;
    };
    let Some(edit) = app.session.editing.as_mut() else {
        return;
    };
    let under = drag.unit.at(&edit.text, offset);
    if offset < drag.origin.start {
        edit.select(drag.origin.end..under.start.min(drag.origin.start));
    } else {
        edit.select(drag.origin.start..under.end.max(drag.origin.end));
    }
}

/// Whether the pointer is over the body of the entity being edited, where
/// it shows a text cursor.
pub(crate) fn is_over_text(app: &App) -> bool {
    let (Some(edit), Some(pointer)) = (&app.session.editing, app.session.pointer) else {
        return false;
    };
    match hit::hit_test(app, pointer) {
        Hit::EntityBody { entity } => entity == edit.entity,
        Hit::GroupLabel { .. }
        | Hit::Handle { .. }
        | Hit::Anchor { .. }
        | Hit::PageContent { .. }
        | Hit::GroupBorder { .. }
        | Hit::Edge { .. }
        | Hit::Empty => false,
    }
}

impl From<TextSelectDrag> for Gesture {
    fn from(drag: TextSelectDrag) -> Self {
        Self::TextSelect(drag)
    }
}
