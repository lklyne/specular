//! Page anchoring (ADR 0031): placement decides whether an entity is hooked
//! to a page. One whose centre lands on a page's body is, and from then on
//! it moves with that page.

use specular_doc::{Document, Entity, Kind, PageAnchor};

use crate::app::page_of;
use crate::geometry;

/// `url` in the form an anchor records and compares: trimmed, with its hash
/// stripped. `None` for a page with no URL yet.
pub(crate) fn canonical_page_url(url: &str) -> Option<String> {
    let document = url.trim().split('#').next().unwrap_or_default();
    (!document.is_empty()).then(|| document.to_owned())
}

/// Whether placement can hook an entity of this kind to a page.
pub const fn anchors_to_pages(kind: &Kind) -> bool {
    match kind {
        Kind::Text(_) | Kind::Drawing(_) | Kind::Shape(_) => true,
        Kind::Page(_) | Kind::File(_) | Kind::Group(_) => false,
    }
}

/// The anchor `entity` gets from where it sits: the frontmost page whose body
/// holds the entity's centre, or `None` on empty canvas. An entity in a
/// group never anchors, because the group already owns its movement.
///
/// The anchor records no scroll offset, so the entity is pinned to the page
/// and does not follow its scroll.
pub fn page_anchor_for(document: &Document, entity: &Entity) -> Option<PageAnchor> {
    if entity.parent.is_some() || !anchors_to_pages(&entity.kind) {
        return None;
    }
    let rect = entity.rect;
    let centre = geometry::origin(rect) + geometry::size(rect) / 2.0;
    document.entities().rev().find_map(|candidate| {
        let page = page_of(candidate)?;
        geometry::contains(candidate.rect, centre).then(|| PageAnchor {
            page_url: canonical_page_url(&page.url),
            ..PageAnchor::new(candidate.id.clone())
        })
    })
}
