//! The move gesture: a press on a body that becomes a drag of the selection,
//! a copy of it with Option held, or a click if the pointer never travels.

use glam::{DVec2, Vec2};
use specular_doc::{EntityId, ItemId, Kind, Rect};

use crate::focus::set_focus;
use crate::live::{self, Start};
use crate::marquee::DRAG_THRESHOLD;
use crate::{App, Effect, PointerInput, clone, geometry, grid, update};

/// A press on a body, and the drag it may become.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveDrag {
    /// The canvas point the press landed on.
    origin: DVec2,
    /// The same point on screen, to tell a drag from a click.
    origin_screen: Vec2,
    /// Where the pressed entity's top-left started. The whole selection
    /// moves by whatever puts this corner on the grid.
    anchor: DVec2,
    /// Whether the pressed entity snaps. Freehand ink does not.
    snaps: bool,
    starts: Vec<Start>,
    click: Click,
    dragged: bool,
    /// How far the selection has moved from where it started.
    delta: DVec2,
    /// Whether Option is held, so the release leaves copies at the pointer
    /// and the originals where they were.
    copying: bool,
}

/// What releasing a press that never became a drag does.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Click {
    /// Nothing more: the press already selected.
    Keep,
    /// Narrows a selection of several to the entity pressed.
    SelectAlone(EntityId),
    /// Enters the page (ADR 0022).
    Enter(EntityId),
}

impl MoveDrag {
    /// Whether the pointer has travelled far enough to be a drag.
    pub fn is_dragging(&self) -> bool {
        self.dragged
    }

    /// Whether releasing now leaves copies.
    pub fn is_copying(&self) -> bool {
        self.dragged && self.copying
    }

    /// Where the copies would land, in canvas space, back-to-front.
    fn copy_rects(&self) -> Vec<Rect> {
        if !self.is_copying() {
            return Vec::new();
        }
        (self.starts.iter())
            .map(|start| start.rect.translated(self.delta.x, self.delta.y))
            .collect()
    }
}

impl App {
    /// The rects an Option-drag would leave copies at, for drawing the
    /// preview. Empty when no copy is being dragged.
    pub fn copy_preview(&self) -> Vec<Rect> {
        match &self.session.gesture {
            Some(crate::Gesture::Move(drag)) => drag.copy_rects(),
            Some(
                crate::Gesture::Resize(_)
                | crate::Gesture::Marquee { .. }
                | crate::Gesture::CommentRegion { .. },
            )
            | None => Vec::new(),
        }
    }
}

/// A press on `pressed`, which is already part of the selection.
pub(crate) fn begin(
    app: &App,
    pressed: &EntityId,
    world: DVec2,
    screen: Vec2,
    click: Click,
) -> Option<MoveDrag> {
    let entity = app.document.entity(pressed)?;
    let snaps = match &entity.kind {
        Kind::Drawing(_) => false,
        Kind::Page(_) | Kind::Text(_) | Kind::File(_) | Kind::Group(_) | Kind::Shape(_) => true,
    };
    Some(MoveDrag {
        origin: world,
        origin_screen: screen,
        anchor: geometry::origin(entity.rect),
        snaps,
        starts: live::starts(&app.document, &app.selection_scope().operands),
        click,
        dragged: false,
        delta: DVec2::ZERO,
        copying: false,
    })
}

/// How far the selection moves for a pointer `raw` away from the press.
/// Shift keeps the move on the axis the pointer has taken further. The
/// anchor lands on the grid, except along an axis Shift is holding still.
fn delta(raw: DVec2, anchor: DVec2, snaps: bool, shift: bool) -> DVec2 {
    let (keep_x, keep_y) = if !shift {
        (true, true)
    } else if raw.x.abs() >= raw.y.abs() {
        (true, false)
    } else {
        (false, true)
    };
    let along = |keep: bool, raw: f64, anchor: f64| {
        if !keep {
            0.0
        } else if snaps {
            grid::snap(anchor + raw) - anchor
        } else {
            raw
        }
    };
    DVec2::new(
        along(keep_x, raw.x, anchor.x),
        along(keep_y, raw.y, anchor.y),
    )
}

/// The pointer moved, or a modifier changed, with `drag` in flight.
pub(crate) fn drag(app: &mut App, drag: &mut MoveDrag, input: &PointerInput) {
    let travel = (input.screen - drag.origin_screen).abs();
    drag.dragged |= travel.max_element() >= DRAG_THRESHOLD;
    if !drag.dragged {
        return;
    }
    let world = app.session.camera.screen_to_world(input.screen).as_dvec2();
    drag.delta = delta(
        world - drag.origin,
        drag.anchor,
        drag.snaps,
        input.modifiers.shift,
    );
    drag.copying = input.modifiers.alt;
    let moved = if drag.copying {
        DVec2::ZERO
    } else {
        drag.delta
    };
    for start in &drag.starts {
        let (rect, kind) = start.moved(moved);
        live::write(&mut app.document, start, rect, kind);
    }
}

/// The button came up. A drag becomes one undo step: the move, or the
/// copies. A press that never travelled is a click.
pub(crate) fn finish(app: &mut App, drag: MoveDrag, effects: &mut Vec<Effect>) {
    if !drag.dragged {
        match drag.click {
            Click::Keep => {}
            Click::SelectAlone(entity) => app.session.selection.set([ItemId::Entity(entity)]),
            Click::Enter(page) => {
                app.session.selection.set([ItemId::Entity(page.clone())]);
                set_focus(app, Some(page), effects);
            }
        }
        return;
    }
    if !drag.copying {
        live::commit(app, &drag.starts);
        return;
    }
    if drag.delta == DVec2::ZERO {
        return;
    }
    let scope = app.selection_scope();
    if let Some(copies) = clone::copies(app, &scope, drag.delta) {
        update::document_step(app, copies.command, effects);
        app.session.selection.set(copies.members);
    }
}

/// The drag was abandoned: everything goes back where it started.
pub(crate) fn cancel(app: &mut App, drag: &MoveDrag) {
    live::restore(&mut app.document, &drag.starts);
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANCHOR: DVec2 = DVec2::new(105.0, 100.0);

    #[test]
    fn the_anchor_lands_on_the_grid() {
        assert_eq!(
            delta(DVec2::new(33.0, -8.0), ANCHOR, true, false),
            DVec2::new(35.0, 0.0)
        );
    }

    #[test]
    fn ink_follows_the_pointer_exactly() {
        assert_eq!(
            delta(DVec2::new(33.0, -8.0), ANCHOR, false, false),
            DVec2::new(33.0, -8.0)
        );
    }

    #[test]
    fn shift_keeps_the_axis_the_pointer_took_further() {
        assert_eq!(
            delta(DVec2::new(33.0, -8.0), ANCHOR, true, true),
            DVec2::new(35.0, 0.0)
        );
        assert_eq!(
            delta(DVec2::new(8.0, 33.0), ANCHOR, true, true),
            DVec2::new(0.0, 40.0)
        );
        // A tie goes to horizontal.
        assert_eq!(
            delta(DVec2::new(30.0, 30.0), ANCHOR, false, true),
            DVec2::new(30.0, 0.0)
        );
    }
}
