//! Comments: the comment tool's gesture, the draft it opens, the composer
//! the draft's text is typed in, and where a comment sits on the canvas.
//!
//! A click with the tool comments on a point, or on the element under it
//! when the click lands on a page. A drag comments on a region, which
//! belongs to a page only when it grabbed something in one. Each opens a
//! draft: an annotation that joins the document, as one undo step, when its
//! text is committed.

mod actions;
mod create;
mod draft;
mod drag;
mod grab;
mod marks;
mod selection;
mod shown;

use glam::DVec2;
use specular_doc::{Annotation, AnnotationAnchor, EntityId, Rect, RegionAnchor};

pub(crate) use actions::{delete, focus, press, resolve, settle};
pub use create::region_annotation;
pub(crate) use draft::{cancel, end, forget, frame, is_over_composer};
pub use drag::CommentDrag;
pub(crate) use drag::{begin, drag, finish, on_element};
pub(crate) use grab::on_region_grab;
pub use grab::{PageGrab, PageRegion};
pub(crate) use marks::at as mark_at;
pub use marks::{
    CommentMark, FOCUS_RING_OUTSET, FOCUS_RING_STROKE, MarkShape, PILL_DIGIT_WIDTH,
    PILL_EDGE_MARGIN, PILL_HEIGHT, PILL_INSET, PILL_WIDTH, REGION_HIT_BAND, REGION_MIN_SIZE,
};
pub(crate) use selection::annotate as annotate_selection;
pub use selection::selection_metadata;

use crate::{
    App, PagePlacement, doc_to_viewport, geometry, left_page, recorded_scroll, viewport_to_doc,
};

/// A rect in a page's CSS pixels, as the viewport sees it, as the canvas rect
/// it covers.
fn css_on_canvas(placement: PagePlacement, css: Rect) -> Rect {
    geometry::rect(
        placement.to_canvas(geometry::origin(css)),
        geometry::size(css) * placement.canvas_per_css(),
    )
}

/// A canvas rect in a page's CSS pixels, as the viewport sees it: what
/// [`css_on_canvas`] undoes.
fn canvas_in_css(placement: PagePlacement, canvas: Rect) -> Rect {
    geometry::rect(
        placement.page_local(geometry::origin(canvas)),
        geometry::size(canvas) / placement.canvas_per_css().max(DVec2::splat(f64::EPSILON)),
    )
}

/// A canvas rect as a rect of `page`'s document: the viewport rect plus how
/// far the page is scrolled. What a region on a page stores.
fn canvas_in_document(app: &App, page: &EntityId, canvas: Rect) -> Option<Rect> {
    let viewport = canvas_in_css(app.page_placement(page)?, canvas);
    Some(viewport_to_doc(viewport, app.page_scroll(page)))
}

/// Where a region annotation sits in canvas space. `None` for the point and
/// element anchors, and for a region whose page is gone.
///
/// A region in a page's document travels with the page and scrolls with it.
pub fn region_on_canvas(app: &App, annotation: &Annotation) -> Option<Rect> {
    match &annotation.anchor {
        AnnotationAnchor::Region(RegionAnchor::Canvas { canvas_rect }) => Some(*canvas_rect),
        AnnotationAnchor::Region(RegionAnchor::Document { doc_rect }) => {
            let page = &annotation.page_anchor.as_ref()?.page_id;
            let viewport = doc_to_viewport(*doc_rect, app.page_scroll(page));
            Some(css_on_canvas(app.page_placement(page)?, viewport))
        }
        AnnotationAnchor::Canvas { .. }
        | AnnotationAnchor::Page { .. }
        | AnnotationAnchor::Element { .. } => None,
    }
}

/// Where the element an annotation is on sat in canvas space when the
/// annotation was made, carried along by how far its page has scrolled since
/// when the annotation recorded the scroll it was made at. `None` for every
/// other anchor, for an element recorded with no box, and when its page is
/// gone.
pub fn element_on_canvas(app: &App, annotation: &Annotation) -> Option<Rect> {
    match &annotation.anchor {
        AnnotationAnchor::Element {
            page_id,
            bounding_box,
            ..
        } => {
            let recorded = (annotation.page_anchor.as_ref()).and_then(recorded_scroll);
            let viewport = match recorded {
                Some(recorded) => {
                    let document = viewport_to_doc((*bounding_box)?, recorded);
                    doc_to_viewport(document, app.page_scroll(page_id))
                }
                None => (*bounding_box)?,
            };
            Some(css_on_canvas(app.page_placement(page_id)?, viewport))
        }
        AnnotationAnchor::Canvas { .. }
        | AnnotationAnchor::Page { .. }
        | AnnotationAnchor::Region(_) => None,
    }
}

/// The canvas rect of the page `annotation` is on, when it has been carried
/// by that page's scroll: a region in the page's document with the page
/// scrolled, or an element whose page has scrolled since it was recorded.
/// What the annotation is clipped to, and hidden outside of. A comment on a
/// page that has not scrolled is drawn whole.
pub fn page_clip(app: &App, annotation: &Annotation) -> Option<Rect> {
    let (page, carried) = match &annotation.anchor {
        AnnotationAnchor::Region(RegionAnchor::Document { .. }) => {
            let page = &annotation.page_anchor.as_ref()?.page_id;
            (page, app.page_scroll(page) != DVec2::ZERO)
        }
        AnnotationAnchor::Element { page_id, .. } => {
            let recorded = recorded_scroll(annotation.page_anchor.as_ref()?)?;
            (page_id, app.page_scroll(page_id) != recorded)
        }
        AnnotationAnchor::Canvas { .. }
        | AnnotationAnchor::Page { .. }
        | AnnotationAnchor::Region(RegionAnchor::Canvas { .. }) => return None,
    };
    if !carried {
        return None;
    }
    Some(app.page_placement(page)?.rect)
}

/// Whether the annotation has scrolled out of its page altogether.
pub fn left_its_page(app: &App, annotation: &Annotation) -> bool {
    let Some(page) = page_clip(app, annotation) else {
        return false;
    };
    let seen = region_on_canvas(app, annotation).or_else(|| element_on_canvas(app, annotation));
    seen.is_some_and(|seen| left_page(page, seen))
}
