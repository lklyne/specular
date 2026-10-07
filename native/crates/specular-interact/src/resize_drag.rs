//! The resize gesture: a handle of one entity or of the whole selection,
//! from the press to the release.

use glam::DVec2;
use specular_core::Modifiers;
use specular_doc::{Drawing, EdgeSide, EntityId, Kind, Rect, Text, WidthMode};

use crate::live::{self, Start};
use crate::{App, Effect, Handle, HandleOwner, PagePlacement, caps, resize, strokes};

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
            let min = caps::min_size(&entity.kind);
            let rect = resize::resized(start.rect, drag.handle, target, min, lock);
            // A handle that has not moved its edges changes nothing, so a
            // click on one is not an undo step.
            let (rect, kind) = if rect == start.rect {
                (rect, start.kind.clone())
            } else {
                resized_kind(start, drag.handle, rect, lock)
            };
            live::write(&mut app.document, start, rect, kind);
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
        }
    }
}

/// What a single-entity resize to `rect` means for the kinds whose fields
/// follow their size.
fn resized_kind(start: &Start, handle: Handle, rect: Rect, lock: bool) -> (Rect, Option<Kind>) {
    match &start.kind {
        Some(Kind::Drawing(drawing)) => (rect, Some(scaled(drawing, start.rect, rect))),
        Some(Kind::Text(text)) => resized_text(text, start.rect, handle, rect, lock),
        Some(Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Shape(_)) | None => {
            (rect, None)
        }
    }
}

/// A text stores its width; its height follows its content. The left and
/// right handles reflow: the width changes and the text keeps its size. Every
/// other handle scales: the size follows the width, as in `FigJam`.
///
/// Nothing measures text here, so a scaling drag that keeps the ratio writes
/// the scaled height as the best guess, and any other drag leaves the height
/// as it was.
fn resized_text(
    text: &Text,
    start: Rect,
    handle: Handle,
    rect: Rect,
    lock: bool,
) -> (Rect, Option<Kind>) {
    let reflows = match handle {
        Handle::Side(EdgeSide::Left | EdgeSide::Right) => true,
        Handle::Side(EdgeSide::Top | EdgeSide::Bottom) | Handle::Corner(_) => false,
    };
    let size = if reflows || start.width <= 0.0 {
        text.size
    } else {
        let scaled = text.size.unwrap_or(TEXT_SIZE_DEFAULT) * rect.width / start.width;
        Some(crate::grid::round(scaled).clamp(TEXT_SIZE_MIN, TEXT_SIZE_MAX))
    };
    let height = if lock && !reflows {
        rect.height
    } else {
        start.height
    };
    let kind = Kind::Text(Text {
        size,
        // A text that grows with its content would undo the new width.
        width_mode: Some(WidthMode::Fixed),
        ..text.clone()
    });
    (Rect { height, ..rect }, Some(kind))
}

fn scaled(drawing: &Drawing, from: Rect, to: Rect) -> Kind {
    Kind::Drawing(Drawing {
        strokes: strokes::scaled(&drawing.strokes, from, to),
    })
}

/// The button came up: the resize becomes one undo step, and each page that
/// changed size is laid out again, once.
pub(crate) fn finish(app: &mut App, drag: &ResizeDrag, effects: &mut Vec<Effect>) {
    live::commit(app, &drag.starts);
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
