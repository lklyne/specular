//! The resize gesture: a handle of one entity or of the whole selection,
//! from the press to the release.

use glam::DVec2;
use specular_core::Modifiers;
use specular_doc::{Drawing, EdgeSide, EntityId, Kind, Rect, Text, WidthMode};

use crate::live::{self, Start};
use crate::{App, Corner, Effect, Handle, HandleOwner, PagePlacement, caps, edit, resize, strokes};

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
    /// The groups above what is resized, refitted around it as the drag
    /// goes.
    followers: Vec<Start>,
}

impl ResizeDrag {
    /// What is being resized.
    pub fn owner(&self) -> &HandleOwner {
        &self.owner
    }

    /// The handle being dragged.
    pub fn handle(&self) -> Handle {
        self.handle
    }

    /// The rect `id` had at the press, when the drag is resizing it.
    pub(crate) fn start_rect(&self, id: &EntityId) -> Option<Rect> {
        (self.starts.iter())
            .find(|start| start.id == *id)
            .map(|start| start.rect)
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
    let (bounds, starts) = match &owner {
        HandleOwner::Entity(id) => {
            let entity = app.document.entity(id)?;
            (entity.rect, vec![Start::of(entity)])
        }
        HandleOwner::Selection => {
            let scope = app.selection_scope();
            (scope.bounds?, live::starts(&app.document, &scope.operands))
        }
    };
    Some(ResizeDrag {
        followers: live::followers_of(&app.document, &starts),
        owner,
        handle,
        grab: handle.point(bounds) - world,
        bounds,
        starts,
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
            let lock = caps::aspect_mode(&entity.kind).locks(modifiers.shift);
            // An entity already under its kind's floor keeps its size until
            // the drag changes it: an auto-width text is narrower than the
            // floor a fixed one has.
            let size = DVec2::new(start.rect.width, start.rect.height);
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
            let rect = resize::resized(start.rect, drag.handle, target, min, lock);
            // A handle that has not moved its edges changes nothing, so a
            // click on one is not an undo step.
            let (rect, kind) = if rect == start.rect {
                (rect, start.kind.clone())
            } else {
                resized_kind(app, start, drag.handle, rect)
            };
            live::write(&mut app.document, start, rect, kind);
            live::follow(&mut app.document, &drag.followers);
        }
        HandleOwner::Selection => {
            let bounds = resize::resized_bounds(drag.bounds, drag.handle, target);
            for start in &drag.starts {
                let rect = resize::placed(start.rect, drag.bounds, bounds);
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

/// What a single-entity resize to `rect` means for the kinds whose fields
/// follow their size.
fn resized_kind(app: &App, start: &Start, handle: Handle, rect: Rect) -> (Rect, Option<Kind>) {
    match &start.kind {
        Some(Kind::Drawing(drawing)) => (rect, Some(scaled(drawing, start.rect, rect))),
        Some(Kind::Text(text)) => resized_text(app, text, start.rect, handle, rect),
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

/// The button came up: the resize becomes one undo step, and each page that
/// changed size is laid out again, once.
pub(crate) fn finish(app: &mut App, drag: &ResizeDrag, effects: &mut Vec<Effect>) {
    live::commit_following(app, &drag.starts, &drag.followers, |_| Vec::new());
    for start in &drag.starts {
        if let Some(placement) = app.page_placement(&start.id)
            && placement.viewport != PagePlacement::viewport_for(start.rect)
        {
            effects.push(Effect::SetPageViewport {
                page: start.id.clone(),
                viewport: placement.viewport,
            });
        }
    }
}
