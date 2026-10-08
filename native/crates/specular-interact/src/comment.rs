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
use specular_doc::{Annotation, AnnotationAnchor, Rect, RegionAnchor};

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

use crate::{App, PagePlacement, geometry};

/// A rect in a page's CSS pixels as the canvas rect it covers. Page scroll
/// is not tracked, so the page is taken as scrolled to its top-left.
fn css_on_canvas(placement: PagePlacement, css: Rect) -> Rect {
    geometry::rect(
        placement.to_canvas(geometry::origin(css)),
        geometry::size(css) * placement.canvas_per_css(),
    )
}

/// A canvas rect in a page's CSS pixels: what [`css_on_canvas`] undoes.
fn canvas_in_css(placement: PagePlacement, canvas: Rect) -> Rect {
    geometry::rect(
        placement.page_local(geometry::origin(canvas)),
        geometry::size(canvas) / placement.canvas_per_css().max(DVec2::splat(f64::EPSILON)),
    )
}

/// Where a region annotation sits in canvas space. `None` for the point and
/// element anchors, and for a region whose page is gone.
///
/// A region in a page's document travels with the page.
pub fn region_on_canvas(app: &App, annotation: &Annotation) -> Option<Rect> {
    match &annotation.anchor {
        AnnotationAnchor::Region(RegionAnchor::Canvas { canvas_rect }) => Some(*canvas_rect),
        AnnotationAnchor::Region(RegionAnchor::Document { doc_rect }) => {
            let page = &annotation.page_anchor.as_ref()?.page_id;
            Some(css_on_canvas(app.page_placement(page)?, *doc_rect))
        }
        AnnotationAnchor::Canvas { .. }
        | AnnotationAnchor::Page { .. }
        | AnnotationAnchor::Element { .. } => None,
    }
}

/// Where the element an annotation is on sat in canvas space when the
/// annotation was made. `None` for every other anchor, for an element
/// recorded with no box, and when its page is gone.
pub fn element_on_canvas(app: &App, annotation: &Annotation) -> Option<Rect> {
    match &annotation.anchor {
        AnnotationAnchor::Element {
            page_id,
            bounding_box,
            ..
        } => Some(css_on_canvas(
            app.page_placement(page_id)?,
            (*bounding_box)?,
        )),
        AnnotationAnchor::Canvas { .. }
        | AnnotationAnchor::Page { .. }
        | AnnotationAnchor::Region(_) => None,
    }
}
