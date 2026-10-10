//! The resize gesture: a handle of one entity or of the whole selection,
//! from the press to the release.

use glam::DVec2;
use specular_core::Modifiers;
use specular_doc::{Drawing, EdgeSide, EntityId, Kind, Rect, Text, WidthMode};

use crate::guides::{self, GuideCapture};
use crate::live::{self, Start};
use crate::scroll_follow::{Scrolls, shift_for};
use crate::{App, Corner, Handle, HandleOwner, anchor, caps, edit, geometry, resize, strokes};

/// The size a text is drawn at when it has none of its own, and the limits a
/// resize keeps it within.
const TEXT_SIZE_DEFAULT: f64 = 14.0;
const TEXT_SIZE_MIN: f64 = 8.0;
const TEXT_SIZE_MAX: f64 = 256.0;

/// A resize handle being dragged.
#[derive(Debug, Clone, PartialEq)]
pub struct ResizeDrag {
    owner: HandleOwner,
    handle: Handle,
    /// The handle's offset from the pointer at the press, so the rect does
    /// not jump.
    grab: DVec2,
    /// The rect the handles sat around at the press.
    bounds: Rect,
    starts: Vec<Start>,
    /// How far each of `starts` is seen from where it is stored, carried by
    /// its page (see `scroll_follow`). The handles are around what is seen,
    /// so the drag works there and stores each rect this far back.
    shifts: Vec<DVec2>,
    /// The groups above what is resized, refitted around it as the drag
    /// goes.
    followers: Vec<Start>,
    /// The neighbours the moving edges can line up with, as the press found
    /// them.
    guides: GuideCapture,
}

impl ResizeDrag {
    pub(crate) fn guide_capture(&self) -> &GuideCapture {
        &self.guides
    }

    /// What is being resized.
    pub fn owner(&self) -> &HandleOwner {
        &self.owner
    }

    /// The handle being dragged.
    pub fn handle(&self) -> Handle {
        self.handle
    }

    pub(crate) fn starts(&self) -> &[Start] {
        &self.starts
    }

    pub(crate) fn followers(&self) -> &[Start] {
        &self.followers
    }
}

/// A press on `handle` at the canvas point `world`.
pub(crate) fn begin(
    app: &App,
    owner: HandleOwner,
    handle: Handle,
    world: DVec2,
) -> Option<ResizeDrag> {
    let starts = match &owner {
        HandleOwner::Entity(id) => vec![Start::of(app.document.entity(id)?)],
        HandleOwner::Selection => live::starts(&app.document, &app.selection_scope().operands),
    };
    let shifts: Vec<DVec2> = (starts.iter())
        .map(|start| {
            (app.document.entity(&start.id)).map_or(DVec2::ZERO, |entity| shift_for(app, entity))
        })
        .collect();
    let bounds = (starts.iter().zip(&shifts))
        .map(|(start, shift)| seen(start.rect, *shift))
        .reduce(geometry::union)?;
    // Neither the selection nor anything inside what is resized is a
    // neighbour.
    let resized: Vec<EntityId> = starts.iter().map(|start| start.id.clone()).collect();
    let mut excluded = app.scope_of(&resized).operands;
    excluded.extend(app.selection_scope().operands);
    Some(ResizeDrag {
        guides: guides::capture(app, &excluded, &resized),
        followers: live::followers_of(&app.document, &starts),
        owner,
        handle,
        grab: handle.point(bounds) - world,
        bounds,
        starts,
        shifts,
    })
}

/// The pointer is at the canvas point `world` with `drag` in flight.
pub(crate) fn drag(app: &mut App, drag: &ResizeDrag, world: DVec2, modifiers: Modifiers) {
    let target = world + drag.grab;
    match &drag.owner {
        HandleOwner::Entity(id) => {
            let (Some(start), Some(entity)) = (drag.starts.first(), app.document.entity(id)) else {
                return;
            };
            let shift = drag.shifts.first().copied().unwrap_or(DVec2::ZERO);
            let from = seen(start.rect, shift);
            let lock = caps::aspect_mode(&entity.kind).locks(modifiers.shift);
            // An entity already under its kind's floor keeps its size until
            // the drag changes it: an auto-width text is narrower than the
            // floor a fixed one has.
            let size = DVec2::new(from.width, from.height);
            let min = caps::min_size(&entity.kind).min(size);
            // A text's height is its content's: only its width has a floor,
            // and a handle that reflows it has no ratio to keep.
            let (lock, min) = match &entity.kind {
                Kind::Text(_) => (lock && !reflows(drag.handle), DVec2::new(min.x, 0.0)),
                Kind::Page(_)
                | Kind::File(_)
                | Kind::Group(_)
                | Kind::Drawing(_)
                | Kind::Shape(_) => (lock, min),
            };
            let rect = resize::resized(from, drag.handle, target, min, lock);
            // A handle that has not moved its edges changes nothing, so a
            // click on one is not an undo step.
            let (rect, kind) = if rect == from {
                (start.rect, start.kind.clone())
            } else {
                let (rect, kind) = resized_kind(app, start, from, drag.handle, rect);
                (seen(rect, -shift), kind)
            };
            live::write(&mut app.document, start, rect, kind);
            live::follow(&mut app.document, &drag.followers);
        }
        HandleOwner::Selection => {
            let bounds = resize::resized_bounds(drag.bounds, drag.handle, target);
            for (start, shift) in drag.starts.iter().zip(&drag.shifts) {
                let rect = resize::placed(seen(start.rect, *shift), drag.bounds, bounds);
                let rect = seen(rect, -*shift);
                let kind = match &start.kind {
                    Some(Kind::Drawing(drawing)) => Some(scaled(drawing, start.rect, rect)),
                    Some(
                        Kind::Page(_)
                        | Kind::Text(_)
                        | Kind::File(_)
                        | Kind::Group(_)
                        | Kind::Shape(_),
                    )
                    | None => None,
                };
                live::write(&mut app.document, start, rect, kind);
            }
            live::follow(&mut app.document, &drag.followers);
        }
    }
}

/// A stored rect where it is seen, `shift` away.
fn seen(rect: Rect, shift: DVec2) -> Rect {
    rect.translated(-shift.x, -shift.y)
}

/// What a single-entity resize from `from` to `rect`, both as seen, means
/// for the kinds whose fields follow their size.
fn resized_kind(
    app: &App,
    start: &Start,
    from: Rect,
    handle: Handle,
    rect: Rect,
) -> (Rect, Option<Kind>) {
    match &start.kind {
        // The points are stored, so they are scaled between stored rects,
        // which lie the same way apart as the seen ones.
        Some(Kind::Drawing(drawing)) => {
            let stored = rect.translated(start.rect.x - from.x, start.rect.y - from.y);
            (rect, Some(scaled(drawing, start.rect, stored)))
        }
        Some(Kind::Text(text)) => resized_text(app, text, from, handle, rect),
        Some(Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Shape(_)) | None => {
            (rect, None)
        }
    }
}

/// A text stores its width; its height follows its content. The left and
/// right handles reflow: the width changes and the text keeps its size. Every
/// other handle scales: the size follows the width, as in `FigJam`.
///
/// Either way the height is measured from the text as it is now set, and the
/// edge the handle is not moving stays where it was.
fn resized_text(
    app: &App,
    text: &Text,
    start: Rect,
    handle: Handle,
    rect: Rect,
) -> (Rect, Option<Kind>) {
    let size = if reflows(handle) || start.width <= 0.0 {
        text.size
    } else {
        let scaled = text.size.unwrap_or(TEXT_SIZE_DEFAULT) * rect.width / start.width;
        Some(crate::grid::round(scaled).clamp(TEXT_SIZE_MIN, TEXT_SIZE_MAX))
    };
    let text = Text {
        size,
        // A text that grows with its content would undo the new width.
        width_mode: Some(WidthMode::Fixed),
        ..text.clone()
    };
    let height = edit::fitted(app, rect, &text).height;
    let moves_top = matches!(
        handle,
        Handle::Side(EdgeSide::Top) | Handle::Corner(Corner::TopLeft | Corner::TopRight)
    );
    let y = if moves_top {
        start.y + start.height - height
    } else {
        start.y
    };
    (Rect { y, height, ..rect }, Some(Kind::Text(text)))
}

/// Whether `handle` changes a text's width and leaves its type size alone.
fn reflows(handle: Handle) -> bool {
    match handle {
        Handle::Side(EdgeSide::Left | EdgeSide::Right) => true,
        Handle::Side(EdgeSide::Top | EdgeSide::Bottom) | Handle::Corner(_) => false,
    }
}

fn scaled(drawing: &Drawing, from: Rect, to: Rect) -> Kind {
    Kind::Drawing(Drawing {
        strokes: strokes::scaled(&drawing.strokes, from, to),
    })
}

/// The button came up: the resize becomes one undo step. A page was laid
/// out at each size on the way (`pages::follow_resize`). What was resized
/// while its page had carried it is stored where it is seen, in the same
/// step (the scroll rebase), so its rect means what the handles showed.
pub(crate) fn finish(app: &mut App, drag: &ResizeDrag) {
    let scrolls = Scrolls::of(app);
    live::commit_following(app, &drag.starts, &drag.followers, |app| {
        let resized: Vec<EntityId> = (drag.starts.iter())
            .filter(|start| {
                (app.document.entity(&start.id)).is_some_and(|entity| entity.rect != start.rect)
            })
            .map(|start| start.id.clone())
            .collect();
        anchor::rebase(&app.document, &scrolls, &resized)
    });
}
