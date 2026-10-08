//! The move gesture: a press on a body that becomes a drag of the selection,
//! a copy of it with Option held, or a click if the pointer never travels.

use glam::{DVec2, Vec2};
use specular_core::Modifiers;
use specular_doc::{Command, EdgeId, EntityId, ItemId, Kind, Rect};

use crate::focus::set_focus;
use crate::live::{self, Start};
use crate::marquee::DRAG_THRESHOLD;
use crate::{App, Effect, PointerInput, anchor, clone, geometry, grid, group_drop, update};

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
    /// The groups above what moves, refitted around it as the drag goes.
    followers: Vec<Start>,
    /// Every group's rect as the drag began, which the drop target is read
    /// from while the groups follow their members.
    group_rects: Vec<(EntityId, Rect)>,
    click: Click,
    dragged: bool,
    /// How far the selection has moved from where it started.
    delta: DVec2,
    /// Whether Option is held, so the release leaves copies at the pointer
    /// and the originals where they were.
    copying: bool,
    /// The group a release now would drop the members into. `None` over no
    /// group, which takes them out of theirs, and while Option, Command or
    /// Control is held, which leaves membership alone.
    drop_target: Option<EntityId>,
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
    /// Selects the edge the press landed on, which crosses the entity a
    /// drag would have moved.
    SelectEdge(EdgeId),
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

    /// What releasing now would copy, and how far from the originals.
    fn copy_preview(&self) -> Option<CopyPreview> {
        self.is_copying().then(|| CopyPreview {
            entities: self.starts.iter().map(|start| start.id.clone()).collect(),
            delta: self.delta,
        })
    }
}

/// The copies an Option-drag would leave if it were released now.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyPreview {
    /// The entities being copied, back-to-front.
    pub entities: Vec<EntityId>,
    /// How far from its original each copy lands, in canvas units.
    pub delta: DVec2,
}

impl App {
    /// The group the items being dragged would be dropped into if released
    /// now, for outlining it. `None` when the drag is a copy, Command or
    /// Control is held, or the pointer is over no group.
    pub fn group_drop_target(&self) -> Option<&EntityId> {
        match &self.session.gesture {
            Some(crate::Gesture::Move(drag)) if drag.dragged => drag.drop_target.as_ref(),
            Some(
                crate::Gesture::Move(_)
                | crate::Gesture::Resize(_)
                | crate::Gesture::Marquee { .. }
                | crate::Gesture::CommentRegion { .. }
                | crate::Gesture::Place(_)
                | crate::Gesture::Draw(_)
                | crate::Gesture::TextSelect(_)
                | crate::Gesture::EdgeDrag(_),
            )
            | None => None,
        }
    }

    /// What an Option-drag would copy and where, for drawing the preview.
    /// `None` when no copy is being dragged.
    pub fn copy_preview(&self) -> Option<CopyPreview> {
        match &self.session.gesture {
            Some(crate::Gesture::Move(drag)) => drag.copy_preview(),
            Some(
                crate::Gesture::Resize(_)
                | crate::Gesture::Marquee { .. }
                | crate::Gesture::CommentRegion { .. }
                | crate::Gesture::Place(_)
                | crate::Gesture::Draw(_)
                | crate::Gesture::TextSelect(_)
                | crate::Gesture::EdgeDrag(_),
            )
            | None => None,
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
    let starts = live::starts(&app.document, &app.selection_scope().operands);
    Some(MoveDrag {
        followers: live::followers_of(&app.document, &starts),
        group_rects: group_drop::group_rects(&app.document),
        origin: world,
        origin_screen: screen,
        anchor: geometry::origin(entity.rect),
        snaps,
        starts,
        click,
        dragged: false,
        delta: DVec2::ZERO,
        copying: false,
        drop_target: None,
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
    let pinned = input.modifiers.meta || input.modifiers.control;
    drag.drop_target = if drag.copying || pinned {
        None
    } else {
        let travelling: Vec<EntityId> = drag.starts.iter().map(|start| start.id.clone()).collect();
        group_drop::drop_target(&app.document, &drag.group_rects, world, &travelling)
    };
    let moved = if drag.copying {
        DVec2::ZERO
    } else {
        drag.delta
    };
    for start in &drag.starts {
        let (rect, kind) = start.moved(moved);
        live::write(&mut app.document, start, rect, kind);
    }
    if drag.copying {
        live::restore(&mut app.document, &drag.followers);
    } else {
        live::follow(&mut app.document, &drag.followers);
    }
}

/// The button came up. A drag becomes one undo step: the move, or the
/// copies. A press that never travelled is a click.
pub(crate) fn finish(
    app: &mut App,
    drag: MoveDrag,
    modifiers: Modifiers,
    effects: &mut Vec<Effect>,
) {
    if !drag.dragged {
        match drag.click {
            Click::Keep => {}
            Click::SelectAlone(entity) => app.session.selection.set([ItemId::Entity(entity)]),
            Click::SelectEdge(edge) => app.session.selection.set([ItemId::Edge(edge)]),
            Click::Enter(page) => {
                app.session.selection.set([ItemId::Entity(page.clone())]);
                set_focus(app, Some(page), effects);
            }
        }
        return;
    }
    // Command or Control at the release keeps every anchor and every
    // membership as it was.
    let rebind = !(modifiers.meta || modifiers.control);
    if !drag.copying {
        let scope = app.selection_scope();
        live::commit_following(app, &drag.starts, &drag.followers, |app| {
            if !rebind {
                return Vec::new();
            }
            // Dropped into or out of a group, then hooked to whatever page
            // they sit on from there: a grouped item never is.
            let mut commands =
                group_drop::reparent(&app.document, &scope.members, drag.drop_target.as_ref());
            let mut after = app.document.clone();
            if after.apply(Command::Batch(commands.clone())).is_ok() {
                commands.extend(anchor::reanchor(&after, &scope.members, &scope.operands));
            }
            commands
        });
        return;
    }
    if drag.delta == DVec2::ZERO {
        return;
    }
    let scope = app.selection_scope();
    if let Some(copies) = clone::copies(app, &scope, drag.delta) {
        let command = if rebind {
            anchor::placed_copies(&mut app.document, copies.command)
        } else {
            copies.command
        };
        update::document_step(app, command, effects);
        app.session.selection.set(copies.members);
    }
}

/// The drag was abandoned: everything goes back where it started.
pub(crate) fn cancel(app: &mut App, drag: &MoveDrag) {
    live::restore(&mut app.document, &drag.starts);
    live::restore(&mut app.document, &drag.followers);
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
