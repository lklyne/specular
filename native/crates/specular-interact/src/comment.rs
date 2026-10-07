//! Comment regions: creating one from a drag, and where one sits on the
//! canvas.

use glam::DVec2;
use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationId, AnnotationStatus, Author, Command, EntityId,
    JsonMap, PageAnchor, Rect, RegionAnchor,
};

use crate::anchor::canonical_page_url;
use crate::app::page_of;
use crate::gesture::apply_step;
use crate::{App, geometry, time};

/// A comment drag shorter than this many logical pixels is a click and
/// creates nothing.
pub(crate) const MIN_COMMENT_DRAG: f32 = 4.0;

/// A new, empty region comment by the user.
pub fn region_annotation(
    id: AnnotationId,
    created_at: String,
    region: RegionAnchor,
    page_anchor: Option<PageAnchor>,
) -> Annotation {
    Annotation {
        id,
        anchor: AnnotationAnchor::Region(region),
        author: Author::User,
        text: String::new(),
        status: AnnotationStatus::Pending,
        replies: Vec::new(),
        created_at,
        element_name: None,
        page_anchor,
        metadata: None,
        extra: JsonMap::new(),
    }
}

/// Where a region annotation sits in canvas space. `None` for the point and
/// element anchors, and for a region whose page is gone.
///
/// A region in a page's document travels with the page. Page scroll is not
/// tracked yet, so the region is placed as if the page were scrolled to its
/// top-left.
pub fn region_on_canvas(app: &App, annotation: &Annotation) -> Option<Rect> {
    match &annotation.anchor {
        AnnotationAnchor::Region(RegionAnchor::Canvas { canvas_rect }) => Some(*canvas_rect),
        AnnotationAnchor::Region(RegionAnchor::Document { doc_rect }) => {
            let page = &annotation.page_anchor.as_ref()?.page_id;
            let placement = app.page_placement(page)?;
            Some(geometry::rect(
                placement.to_canvas(geometry::origin(*doc_rect)),
                geometry::size(*doc_rect) * placement.canvas_per_css(),
            ))
        }
        AnnotationAnchor::Canvas { .. }
        | AnnotationAnchor::Page { .. }
        | AnnotationAnchor::Element { .. } => None,
    }
}

/// Adds a comment on the canvas-space `region` as one undo step, bound to
/// `page` when that is still a page and to the canvas otherwise.
pub(crate) fn create_region(app: &mut App, region: Rect, page: Option<EntityId>) {
    let bound = page.and_then(|id| {
        let placement = app.page_placement(&id)?;
        let url = page_of(app.document.entity(&id)?)?.url.clone();
        Some((id, placement, url))
    });
    let (anchor, page_anchor) = match bound {
        Some((id, placement, url)) => {
            let doc_rect = geometry::rect(
                placement.page_local(geometry::origin(region)),
                geometry::size(region) / placement.canvas_per_css().max(DVec2::splat(f64::EPSILON)),
            );
            let page_anchor = PageAnchor {
                page_url: canonical_page_url(&url),
                ..PageAnchor::new(id)
            };
            (RegionAnchor::Document { doc_rect }, Some(page_anchor))
        }
        None => (
            RegionAnchor::Canvas {
                canvas_rect: region,
            },
            None,
        ),
    };
    let annotation = region_annotation(
        app.fresh_annotation_id(),
        time::iso8601(app.session.now_ms),
        anchor,
        page_anchor,
    );
    let at = app.document.annotations().len();
    apply_step(
        app,
        Command::InsertAnnotation {
            annotation: Box::new(annotation),
            at,
        },
    );
}
