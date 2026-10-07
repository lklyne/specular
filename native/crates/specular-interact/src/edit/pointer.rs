//! The pointer in the text being edited: a click places the caret, a double
//! click takes a word and a triple click a paragraph, and dragging from any
//! of them extends the selection by the same unit.

use std::ops::Range;

use glam::DVec2;

use super::buffer::Target;
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

/// How close to the viewport's edge, in logical pixels, a selection drag
/// starts to pan the canvas.
const PAN_EDGE: f32 = 24.0;
/// How fast a selection drag scrolls, in logical pixels a second for each
/// pixel the pointer is past the edge, and the most it reaches.
const SCROLL_RATE: f32 = 12.0;
const SCROLL_MAX: f32 = 1500.0;
/// The longest stretch of time one tick scrolls for, so a stalled loop does
/// not jump.
const TICK_MAX_MS: u64 = 100;

/// How far past the span from `low` to `high` the value `at` is: negative
/// below it, positive above, zero inside.
fn past(at: f32, low: f32, high: f32) -> f32 {
    (at - high).max(0.0) + (at - low).min(0.0)
}

/// The clock moved by `elapsed_ms` with a selection drag in flight. A
/// pointer held past the top or bottom of a Document's window scrolls it,
/// and one held at the viewport's edge pans the canvas, when the text goes
/// on that way. The selection follows.
pub(crate) fn autoscroll(app: &mut App, elapsed_ms: u64) {
    let (Some(Gesture::TextSelect(drag)), Some(pointer)) =
        (app.session.gesture.clone(), app.session.pointer)
    else {
        return;
    };
    let Some(edit) = &app.session.editing else {
        return;
    };
    let Some(rect) = app.document.entity(&edit.entity).map(|entity| entity.rect) else {
        return;
    };
    let entity = edit.entity.clone();
    let camera = app.session.camera;
    let seconds = elapsed_ms.min(TICK_MAX_MS) as f32 / 1000.0;
    let step = |past: f32| (past * SCROLL_RATE).clamp(-SCROLL_MAX, SCROLL_MAX) * seconds;
    let corner = |x: f64, y: f64| camera.world_to_screen(glam::Vec2::new(x as f32, y as f32));
    let (top_left, bottom_right) = (
        corner(rect.x, rect.y),
        corner(rect.x + rect.width, rect.y + rect.height),
    );
    if edit.target == Target::Note {
        let by = step(past(pointer.y, top_left.y, bottom_right.y));
        if by == 0.0 {
            return;
        }
        let offset = app.session.notes.scroll(&entity) + by / camera.zoom.max(f32::EPSILON);
        crate::notes::scroll_to(app, &entity, offset);
    } else {
        let viewport = app.session.viewport;
        let mut by = glam::Vec2::new(
            step(past(pointer.x, PAN_EDGE, viewport.x - PAN_EDGE)),
            step(past(pointer.y, PAN_EDGE, viewport.y - PAN_EDGE)),
        );
        // Only towards text that is off screen.
        if (by.x < 0.0 && top_left.x >= 0.0) || (by.x > 0.0 && bottom_right.x <= viewport.x) {
            by.x = 0.0;
        }
        if (by.y < 0.0 && top_left.y >= 0.0) || (by.y > 0.0 && bottom_right.y <= viewport.y) {
            by.y = 0.0;
        }
        if by == glam::Vec2::ZERO {
            return;
        }
        app.session.camera.pan -= by;
    }
    let world = app.session.camera.screen_to_world(pointer).as_dvec2();
    self::drag(app, &drag, world);
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
