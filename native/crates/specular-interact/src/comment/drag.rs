//! The comment tool's gesture: a click comments on a point or on the element
//! under it, and a drag comments on a region.

use glam::{DVec2, Vec2};
use specular_core::PageElement;
use specular_doc::{EntityId, Rect};

use super::{create, draft, grab};
use crate::{App, Effect, Gesture, Hit, PointerInput, Session, geometry, hit};

/// A press with the comment tool is a click until the pointer has travelled
/// this many logical pixels, and a region after.
const MIN_COMMENT_DRAG: f32 = 4.0;

/// A press with the comment tool, up to its release.
#[derive(Debug, Clone, PartialEq)]
pub struct CommentDrag {
    /// The canvas point the press landed on.
    start: DVec2,
    /// The same point on screen, to tell a drag from a click.
    start_screen: Vec2,
    /// The canvas point the pointer is at.
    current: DVec2,
    /// Whether the pointer has travelled far enough to be a drag.
    dragged: bool,
}

impl CommentDrag {
    /// The region dragged out so far, in canvas space. `None` while the
    /// press is still a click.
    pub fn region(&self) -> Option<Rect> {
        (self.dragged).then(|| geometry::spanning(self.start, self.current))
    }
}

/// A press with the comment tool at `input`.
pub(crate) fn begin(app: &App, input: &PointerInput) -> CommentDrag {
    let world = app.session.camera.screen_to_world(input.screen).as_dvec2();
    CommentDrag {
        start: world,
        start_screen: input.screen,
        current: world,
        dragged: false,
    }
}

/// The pointer moved to `screen` with `drag` in flight.
pub(crate) fn drag(app: &App, drag: &mut CommentDrag, screen: Vec2) {
    drag.current = app.session.camera.screen_to_world(screen).as_dvec2();
    drag.dragged |= (screen - drag.start_screen).length() >= MIN_COMMENT_DRAG;
}

/// The button came up. A drag opens a region draft, at once when it lies
/// over no page; over pages, the shell is first asked what it grabbed. A
/// click on a page asks for the element there, and a click anywhere else
/// opens a draft on that point.
pub(crate) fn finish(app: &mut App, drag: &CommentDrag, effects: &mut Vec<Effect>) {
    let Some(region) = drag.region() else {
        return click(app, drag, effects);
    };
    let pages = grab::pages_under(app, region);
    if pages.is_empty() {
        let made = create::canvas_region(app, region);
        draft::open(app, made, effects);
    } else {
        effects.push(Effect::QueryRegionGrab { region, pages });
    }
}

fn click(app: &mut App, drag: &CommentDrag, effects: &mut Vec<Effect>) {
    match hit::body_at(app, drag.start_screen) {
        Hit::PageContent { page, local } => {
            effects.push(Effect::QueryElement { page, point: local });
        }
        Hit::GroupLabel { .. }
        | Hit::Handle { .. }
        | Hit::Anchor { .. }
        | Hit::Comment { .. }
        | Hit::EntityBody { .. }
        | Hit::GroupBorder { .. }
        | Hit::Layout(_)
        | Hit::Edge { .. }
        | Hit::Panel { .. }
        | Hit::Empty => {
            let made = create::canvas_point(app, drag.start);
            draft::open(app, made, effects);
        }
    }
}

/// The shell said what `page` has under `point`, in its CSS pixels. An
/// element becomes a draft on it. With none there, the click becomes a
/// draft on that point of the canvas.
///
/// An answer that finds a drag in flight is dropped, as is one whose page
/// has gone.
pub(crate) fn on_element(
    app: &mut App,
    page: &EntityId,
    point: Vec2,
    element: Option<PageElement>,
    effects: &mut Vec<Effect>,
) {
    if app.session.gesture.is_some() {
        return;
    }
    let (Some(binding), Some(placement)) =
        (create::page_anchor(app, page), app.page_placement(page))
    else {
        return;
    };
    let made = match element {
        Some(element) => create::element(app, binding, element),
        None => create::canvas_point(app, placement.to_canvas(point.as_dvec2())),
    };
    draft::open(app, made, effects);
}

impl Session {
    /// The region the comment tool is dragging out, in canvas space. `None`
    /// until the press has travelled far enough to be a drag.
    pub fn comment_preview(&self) -> Option<Rect> {
        match &self.gesture {
            Some(Gesture::Comment(drag)) => drag.region(),
            Some(
                Gesture::Move(_)
                | Gesture::Resize(_)
                | Gesture::Marquee { .. }
                | Gesture::Place(_)
                | Gesture::Draw(_)
                | Gesture::TextSelect(_)
                | Gesture::EdgeDrag(_)
                | Gesture::Line(_),
            )
            | None => None,
        }
    }
}
