//! Bringing something into view: what a click on a sidebar row does to the
//! camera, as `focusCanvasBounds` does in the Electron app.

use glam::DVec2;
use specular_doc::{Annotation, AnnotationAnchor, AnnotationId, ItemId, Rect};

use crate::update::{drop_dangling, verb};
use crate::viewport::area;
use crate::{App, Effect, comment, element_on_canvas, geometry, region_on_canvas};

/// How big a comment on a point of the canvas is taken to be when it is
/// brought into view: `POINT_FOCUS_SIZE`.
const POINT_SIZE: f64 = 100.0;

/// Selects `select` and brings `focus` into view, unless a drag is in
/// flight.
pub(crate) fn items(app: &mut App, select: Vec<ItemId>, focus: &ItemId, effects: &mut Vec<Effect>) {
    verb(app, effects, |app, effects| {
        app.session.selection.set(select);
        drop_dangling(app, effects);
        if let ItemId::Entity(id) = focus
            && let Some(entity) = app.document.entity(id)
        {
            bring_into_view(app, entity.rect);
        }
    });
}

/// Gives the comment `id` the focus and brings what it is on into view.
pub(crate) fn comment(app: &mut App, id: &AnnotationId, effects: &mut Vec<Effect>) {
    verb(app, effects, |app, _| {
        comment::focus(app, Some(id));
        if app.session.focused_comment.as_ref() != Some(id) {
            return;
        }
        let bounds =
            (app.document.annotation(id)).and_then(|annotation| comment_bounds(app, annotation));
        if let Some(bounds) = bounds {
            bring_into_view(app, bounds);
        }
    });
}

/// Where a comment is on the canvas: its region or element, the page it is
/// on, or a square about its point.
fn comment_bounds(app: &App, annotation: &Annotation) -> Option<Rect> {
    let page = |id| app.page_placement(id).map(|placement| placement.rect);
    match &annotation.anchor {
        AnnotationAnchor::Canvas { canvas_x, canvas_y } => Some(Rect::new(
            canvas_x - POINT_SIZE / 2.0,
            canvas_y - POINT_SIZE / 2.0,
            POINT_SIZE,
            POINT_SIZE,
        )),
        AnnotationAnchor::Region(_) => region_on_canvas(app, annotation),
        AnnotationAnchor::Element { page_id, .. } => {
            element_on_canvas(app, annotation).or_else(|| page(page_id))
        }
        AnnotationAnchor::Page { page_id, .. } => page(page_id),
    }
}

/// Leaves the camera alone when `bounds` is all in the part of the viewport
/// the toolbar and the sidebar leave free, and otherwise pans, at the zoom
/// the camera has, to put its centre in the middle of that part.
pub(crate) fn bring_into_view(app: &mut App, bounds: Rect) {
    let free = area(app);
    if free.is_empty() {
        return;
    }
    let camera = &mut app.session.camera;
    let zoom = f64::from(camera.zoom);
    let low = geometry::origin(bounds) * zoom + camera.pan.as_dvec2();
    let high = low + geometry::size(bounds) * zoom;
    if low.cmpge(free.min).all() && high.cmple(free.max()).all() {
        return;
    }
    let centre = geometry::origin(bounds) + geometry::size(bounds) / 2.0;
    let pan: DVec2 = (free.centre() - centre * zoom).round();
    camera.pan = pan.as_vec2();
}
