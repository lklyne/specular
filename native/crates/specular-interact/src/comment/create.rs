//! Making a draft of each anchor form: a canvas point, a page's element, a
//! region of the canvas and a region of a page's document.

use glam::DVec2;
use specular_core::PageElement;
use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationId, EntityId, PageAnchor, Rect, RegionAnchor,
};

use crate::anchor::canonical_page_url;
use crate::app::page_of;
use crate::{App, time};

/// A new, empty region comment by the user.
pub fn region_annotation(
    id: AnnotationId,
    created_at: String,
    region: RegionAnchor,
    page_anchor: Option<PageAnchor>,
) -> Annotation {
    Annotation {
        page_anchor,
        ..Annotation::new(id, AnnotationAnchor::Region(region), created_at)
    }
}

/// A new, empty comment by the user at `anchor`, made now under an id
/// nothing uses.
fn fresh(app: &mut App, anchor: AnnotationAnchor) -> Annotation {
    let created_at = time::iso8601(app.session.now_ms);
    Annotation::new(app.fresh_annotation_id(), anchor, created_at)
}

/// What binds a comment to `page`'s document as it is now, or `None` when
/// `page` is not a page.
pub(super) fn page_anchor(app: &App, page: &EntityId) -> Option<PageAnchor> {
    let url = &page_of(app.document.entity(page)?)?.url;
    Some(PageAnchor {
        page_url: canonical_page_url(url),
        ..PageAnchor::new(page.clone())
    })
}

/// A comment on the canvas point `at`. It is bound to no page, whatever is
/// under it.
pub(super) fn canvas_point(app: &mut App, at: DVec2) -> Annotation {
    fresh(
        app,
        AnnotationAnchor::Canvas {
            canvas_x: at.x,
            canvas_y: at.y,
        },
    )
}

/// A comment on `element` of the page that `binding` names.
///
/// The element's box is the page's, in its viewport, so the binding records
/// the scroll it was taken at and the box follows the page from there.
pub(super) fn element(app: &mut App, binding: PageAnchor, element: PageElement) -> Annotation {
    let bounds = element.bounding_box;
    let scroll = app.page_scroll(&binding.page_id);
    let binding = PageAnchor {
        scroll_x: Some(scroll.x),
        scroll_y: Some(scroll.y),
        ..binding
    };
    let anchor = AnnotationAnchor::Element {
        page_id: binding.page_id.clone(),
        selector: element.selector,
        element_path: element.element_path,
        bounding_box: Some(Rect::new(
            f64::from(bounds.x),
            f64::from(bounds.y),
            f64::from(bounds.width),
            f64::from(bounds.height),
        )),
    };
    Annotation {
        page_anchor: Some(binding),
        ..fresh(app, anchor)
    }
}

/// A comment on the canvas-space `region`, bound to no page.
pub(super) fn canvas_region(app: &mut App, region: Rect) -> Annotation {
    let anchor = RegionAnchor::Canvas {
        canvas_rect: region,
    };
    fresh(app, AnnotationAnchor::Region(anchor))
}

/// A comment on the canvas-space `region` as a region of `page`'s document,
/// which it then travels with. `None` when `page` is not a page.
///
/// The rect and the binding are made together, so a region never has one
/// without the other.
pub(super) fn page_region(app: &mut App, page: &EntityId, region: Rect) -> Option<Annotation> {
    let binding = page_anchor(app, page)?;
    let doc_rect = super::canvas_in_document(app, page, region)?;
    let anchor = AnnotationAnchor::Region(RegionAnchor::Document { doc_rect });
    Some(Annotation {
        page_anchor: Some(binding),
        ..fresh(app, anchor)
    })
}
