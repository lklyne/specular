//! [`Gesture`]: the drag in flight, and what moving, releasing and
//! cancelling it does.

use glam::{DVec2, Vec2};
use specular_doc::{Command, Document, EntityId, Rect};

use crate::{App, Corner, Effect, PagePlacement, comment, geometry, handles};

/// A pointer drag between a press and its release. It owns the pointer: no
/// page sees the moves or the release.
///
/// Match on this without a wildcard arm, so a new gesture makes the compiler
/// list every place that must handle it.
#[derive(Debug, Clone, PartialEq)]
pub enum Gesture {
    /// Moving entities.
    Move {
        /// The canvas point the drag started at.
        origin: DVec2,
        /// Each moved entity and the rect it started with.
        items: Vec<(EntityId, Rect)>,
    },
    /// Dragging a resize handle.
    Resize {
        /// The entity being resized.
        entity: EntityId,
        /// The handle being dragged.
        corner: Corner,
        /// The handle's offset from the pointer at the press, so the rect
        /// does not jump.
        grab: DVec2,
        /// The rect the entity started with.
        start: Rect,
        /// The smallest size the drag may produce.
        min_size: DVec2,
    },
    /// Dragging out a comment region.
    CommentRegion {
        /// The canvas point the drag started at.
        start: DVec2,
        /// The same point on screen, to tell a drag from a click.
        start_screen: Vec2,
        /// The canvas point the pointer is at.
        current: DVec2,
        /// The page the drag started over, which the comment binds to.
        page: Option<EntityId>,
    },
}

/// The pointer moved to `screen` mid-drag.
pub(crate) fn drag(app: &mut App, screen: Vec2) {
    let world = app.session.camera.screen_to_world(screen).as_dvec2();
    match &mut app.session.gesture {
        None => {}
        Some(Gesture::Move { origin, items }) => {
            let delta = world - *origin;
            for (id, start) in items.iter() {
                set_rect(&mut app.document, id, start.translated(delta.x, delta.y));
            }
        }
        Some(Gesture::Resize {
            entity,
            corner,
            grab,
            start,
            min_size,
        }) => {
            let rect = handles::resized(*start, *corner, world + *grab, *min_size);
            set_rect(&mut app.document, entity, rect);
        }
        Some(Gesture::CommentRegion { current, .. }) => *current = world,
    }
}

/// The button came up at `screen`, ending `gesture`. A move or resize becomes
/// one undo step; a resized page is re-laid-out; a comment drag long enough
/// to not be a click creates its annotation.
pub(crate) fn finish(app: &mut App, gesture: Gesture, screen: Vec2, effects: &mut Vec<Effect>) {
    match gesture {
        Gesture::Move { items, .. } => commit_rects(app, &items),
        Gesture::Resize { entity, start, .. } => {
            let laid_out_at = PagePlacement::viewport_for(start);
            commit_rects(app, &[(entity.clone(), start)]);
            if let Some(placement) = app.page_placement(&entity)
                && placement.viewport != laid_out_at
            {
                effects.push(Effect::SetPageViewport {
                    page: entity,
                    viewport: placement.viewport,
                });
            }
        }
        Gesture::CommentRegion {
            start,
            start_screen,
            page,
            ..
        } => {
            if (screen - start_screen).length() >= comment::MIN_COMMENT_DRAG {
                let end = app.session.camera.screen_to_world(screen).as_dvec2();
                comment::create_region(app, geometry::spanning(start, end), page);
            }
        }
    }
}

/// Abandons the gesture in flight: a move or resize snaps back and a comment
/// region is dropped.
pub(crate) fn cancel(app: &mut App) {
    match app.session.gesture.take() {
        None | Some(Gesture::CommentRegion { .. }) => {}
        Some(Gesture::Move { items, .. }) => {
            for (id, start) in &items {
                set_rect(&mut app.document, id, *start);
            }
        }
        Some(Gesture::Resize { entity, start, .. }) => {
            set_rect(&mut app.document, &entity, start);
        }
    }
}

/// Runs `command` as one undo step.
pub(crate) fn apply_step(app: &mut App, command: Command) {
    if let Err(error) = app.history.apply(&mut app.document, command) {
        tracing::warn!("command refused: {error}");
    }
}

/// Changes a rect without an undo step, for the frames of a drag.
fn set_rect(document: &mut Document, id: &EntityId, rect: Rect) {
    let command = Command::SetRect {
        id: id.clone(),
        rect,
    };
    if let Err(error) = document.apply(command) {
        tracing::warn!("drag refused: {error}");
    }
}

/// Turns the rects a drag left in the document into one undo step whose
/// inverse restores `starts`.
fn commit_rects(app: &mut App, starts: &[(EntityId, Rect)]) {
    let (mut restore, mut commit) = (Vec::new(), Vec::new());
    for (id, start) in starts {
        let Some(entity) = app.document.entity(id) else {
            continue;
        };
        if entity.rect == *start {
            continue;
        }
        restore.push(Command::SetRect {
            id: id.clone(),
            rect: *start,
        });
        commit.push(Command::SetRect {
            id: id.clone(),
            rect: entity.rect,
        });
    }
    if commit.is_empty() {
        return;
    }
    if let Err(error) = app.document.apply(Command::Batch(restore)) {
        tracing::warn!("drag could not be recorded: {error}");
        return;
    }
    let step = if commit.len() == 1 {
        commit.remove(0)
    } else {
        Command::Batch(commit)
    };
    apply_step(app, step);
}
