//! Scroll-follow and element-follow: an entity hooked to a page tracks the
//! document content it was placed over. Its anchor records the page's scroll
//! at placement, and the element it was placed over with where that element
//! then sat (ADR 0032). It is drawn and hit shifted by how far the page has
//! scrolled and the element has moved since, so a sticky stays on the
//! paragraph it was put beside through a scroll and a reflow.
//!
//! The stored rect stays the truth. The shift is folded into it when the
//! entity is moved, resized or re-anchored (see [`fold`]), which restamps
//! the scroll and the element's place. An anchor with no scroll and no
//! element is pinned to the page's frame and never shifts.
//!
//! Drawing, hit-testing, outlines, marquees, edges and the text editor all
//! read the shifted rect from here, so what is seen is what is grabbed.

use std::borrow::Cow;
use std::collections::HashMap;

use glam::DVec2;
use specular_core::ElementPlace;
use specular_doc::{AnchorElement, Drawing, Entity, EntityId, Kind, PageAnchor, Rect};

use crate::{App, geometry, showing, strokes};

/// How far past its page's top and bottom an item that follows the page is
/// still drawn, fading out, in logical pixels: Electron's `BAND_FADE_MARGIN`.
/// The sides cut hard, since a page scrolls up and down.
pub const PAGE_FADE: f32 = 48.0;

/// A document rect in a page's CSS pixels as the viewport sees it, with the
/// page scrolled to `scroll`.
pub fn doc_to_viewport(doc: Rect, scroll: DVec2) -> Rect {
    doc.translated(-scroll.x, -scroll.y)
}

/// A viewport rect in a page's CSS pixels as the document holds it, with the
/// page scrolled to `scroll`.
pub fn viewport_to_doc(viewport: Rect, scroll: DVec2) -> Rect {
    viewport.translated(scroll.x, scroll.y)
}

/// The scroll an anchor recorded, or `None` for an anchor pinned to the
/// page's frame or hooked to a fixed element, which the scroll does not
/// move.
pub fn recorded_scroll(anchor: &PageAnchor) -> Option<DVec2> {
    if anchor
        .element
        .as_ref()
        .is_some_and(|element| element.viewport_positioned == Some(true))
    {
        return None;
    }
    // Pages scroll vertically, so the vertical offset is what says an anchor
    // follows; a horizontal one alone is a stray field.
    let y = anchor.scroll_y?;
    Some(DVec2::new(anchor.scroll_x.unwrap_or(0.0), y))
}

/// Whether `shown` has left `page` altogether. A rect that only touches the
/// page's edge is still there.
pub fn left_page(page: Rect, shown: Rect) -> bool {
    shown.x > page.x + page.width
        || shown.y > page.y + page.height
        || shown.x + shown.width < page.x
        || shown.y + shown.height < page.y
}

/// How far the element an anchor recorded has moved, in CSS pixels, in the
/// convention of the scroll shift: subtract it from the stored position.
/// Zero for an anchor with no element and for one whose selector finds
/// nothing now, which leaves the item where it is stored.
pub(crate) fn element_shift(anchor: &PageAnchor, places: &HashMap<String, ElementPlace>) -> DVec2 {
    let Some(element) = &anchor.element else {
        return DVec2::ZERO;
    };
    places.get(&element.selector).map_or(DVec2::ZERO, |live| {
        DVec2::new(element.doc_x, element.doc_y) - live.doc.as_dvec2()
    })
}

/// How far an item with `anchor` has moved since the anchor was written, in
/// its page's CSS pixels: the page's scroll since, and its element's travel
/// through the document since. Subtract it from the stored position.
///
/// An element in a fixed or sticky container moves through the document as
/// the page scrolls, so the two cancel and the item stays on it. Until such
/// an element has been found on the page the item is pinned to the frame,
/// which is where a fixed element stays.
pub(crate) fn anchor_shift(
    anchor: &PageAnchor,
    scroll: DVec2,
    places: &HashMap<String, ElementPlace>,
) -> DVec2 {
    let live = (anchor.element.as_ref()).and_then(|element| places.get(&element.selector));
    let pinned = live.is_none()
        && (anchor.element.as_ref())
            .is_some_and(|element| element.viewport_positioned == Some(true));
    let scrolled = match anchor.scroll_y {
        Some(y) if !pinned => scroll - DVec2::new(anchor.scroll_x.unwrap_or(0.0), y),
        Some(_) | None => DVec2::ZERO,
    };
    scrolled + element_shift(anchor, places)
}

/// A page's scroll, where its tracked elements are, and the scale of its
/// CSS pixels, as an anchor reads them. A snapshot, so a step that edits
/// the document can read it.
#[derive(Debug, Clone)]
pub(crate) struct PageScroll {
    live: DVec2,
    per_css: DVec2,
    places: HashMap<String, ElementPlace>,
}

/// The scroll and the tracked elements of every page, as the app has them
/// now.
#[derive(Debug, Clone, Default)]
pub(crate) struct Scrolls(HashMap<EntityId, PageScroll>);

impl Scrolls {
    pub(crate) fn of(app: &App) -> Self {
        Self(
            app.pages()
                .map(|(id, _, placement)| {
                    let scroll = PageScroll {
                        live: app.page_scroll(id),
                        per_css: placement.canvas_per_css(),
                        places: (app.page_state(id))
                            .map(|state| state.elements.clone())
                            .unwrap_or_default(),
                    };
                    (id.clone(), scroll)
                })
                .collect(),
        )
    }

    /// The page's scroll, zero for a page that has not said or is gone.
    pub(crate) fn live(&self, page: &EntityId) -> DVec2 {
        self.0.get(page).map_or(DVec2::ZERO, |scroll| scroll.live)
    }

    /// How far `entity` is shifted, in canvas units.
    pub(crate) fn shift(&self, entity: &Entity) -> DVec2 {
        let Some(anchor) = &entity.anchor else {
            return DVec2::ZERO;
        };
        self.0.get(&anchor.page_id).map_or(DVec2::ZERO, |page| {
            anchor_shift(anchor, page.live, &page.places) * page.per_css
        })
    }
}

/// The anchor `anchor` becomes when its entity's shift is folded into its
/// stored position: the same page, restamped at the page's scroll now and at
/// where its element is now. An anchor that records no scroll keeps none,
/// and an element that cannot be found keeps the place it recorded.
pub(crate) fn restamped(scrolls: &Scrolls, anchor: &PageAnchor) -> PageAnchor {
    let Some(page) = scrolls.0.get(&anchor.page_id) else {
        return anchor.clone();
    };
    let follows = anchor.scroll_y.is_some();
    let element = anchor.element.as_ref().map(|element| {
        (page.places.get(&element.selector)).map_or_else(
            || element.clone(),
            |live| AnchorElement {
                doc_x: f64::from(live.doc.x),
                doc_y: f64::from(live.doc.y),
                ..element.clone()
            },
        )
    });
    PageAnchor {
        scroll_x: follows.then_some(page.live.x).or(anchor.scroll_x),
        scroll_y: follows.then_some(page.live.y),
        element,
        ..anchor.clone()
    }
}

/// `entity` with its shift folded into its stored rect (and the points of a
/// drawing), so what is stored is what is seen. `None` when it has no shift.
pub(crate) fn fold(scrolls: &Scrolls, entity: &Entity) -> Option<Entity> {
    let shift = scrolls.shift(entity);
    (shift != DVec2::ZERO).then(|| shifted(entity, -shift))
}

/// `entity` moved by `delta`, with a drawing's strokes.
fn shifted(entity: &Entity, delta: DVec2) -> Entity {
    let kind = match &entity.kind {
        Kind::Drawing(drawing) => Kind::Drawing(Drawing {
            strokes: strokes::translated(&drawing.strokes, delta),
        }),
        other @ (Kind::Page(_)
        | Kind::Text(_)
        | Kind::File(_)
        | Kind::Group(_)
        | Kind::Shape(_)) => other.clone(),
    };
    Entity {
        rect: entity.rect.translated(delta.x, delta.y),
        kind,
        ..entity.clone()
    }
}

/// `entity` as it is seen, and the rect it shows through.
#[derive(Debug, Clone)]
pub struct Seen<'a> {
    /// The entity shifted by its page's scroll. Borrowed when it has no
    /// shift.
    pub entity: Cow<'a, Entity>,
    /// The page's rect in canvas space, which a shifted entity is clipped
    /// to. `None` for an entity that is not shifted, which is drawn whole.
    pub clip: Option<Rect>,
}

/// `page` grown by the fade's reach above and below: where something that
/// follows the page can still be seen.
fn band(app: &App, page: Rect) -> Rect {
    let reach = f64::from(PAGE_FADE / app.session.camera.zoom.max(f32::EPSILON));
    Rect::new(
        page.x,
        page.y - reach,
        page.width,
        page.height + reach * 2.0,
    )
}

/// Whether `shown`, something carried by `page`'s scroll, is past where the
/// page lets it be seen: off its sides, or beyond the fade above and below.
pub fn out_of_page(app: &App, page: Rect, shown: Rect) -> bool {
    left_page(band(app, page), shown)
}

/// `entity` as it is seen: shifted by its page's scroll and its element's
/// travel, and clipped to the page while it is. `None` when it has left the
/// page and the fade around it altogether, or an item view leaves it out,
/// which hides it and takes it out of hit-testing.
pub fn seen<'a>(app: &App, entity: &'a Entity) -> Option<Seen<'a>> {
    if showing::hides(app, entity) {
        return None;
    }
    let shift = shift_for(app, entity);
    if shift == DVec2::ZERO {
        return Some(Seen {
            entity: Cow::Borrowed(entity),
            clip: None,
        });
    }
    let page = app.page_placement(&entity.anchor.as_ref()?.page_id)?.rect;
    let shown = shifted(entity, -shift);
    (!out_of_page(app, page, shown.rect)).then_some(Seen {
        entity: Cow::Owned(shown),
        clip: Some(page),
    })
}

/// The rect `entity` is seen at, or `None` when it is hidden.
pub fn shown_rect(app: &App, entity: &Entity) -> Option<Rect> {
    if showing::hides(app, entity) {
        return None;
    }
    let shift = shift_for(app, entity);
    if shift == DVec2::ZERO {
        return Some(entity.rect);
    }
    let page = app.page_placement(&entity.anchor.as_ref()?.page_id)?.rect;
    let shown = entity.rect.translated(-shift.x, -shift.y);
    (!out_of_page(app, page, shown)).then_some(shown)
}

/// The rect `entity` is at with its shift taken off, seen or not: where a
/// gesture finds it and where its text is laid out.
pub(crate) fn placed_rect(app: &App, entity: &Entity) -> Rect {
    let shift = shift_for(app, entity);
    entity.rect.translated(-shift.x, -shift.y)
}

/// How far `entity` is shifted, in canvas units.
pub(crate) fn shift_for(app: &App, entity: &Entity) -> DVec2 {
    let Some(anchor) = &entity.anchor else {
        return DVec2::ZERO;
    };
    let Some(placement) = app.page_placement(&anchor.page_id) else {
        return DVec2::ZERO;
    };
    let css = match app.page_state(&anchor.page_id) {
        Some(state) => anchor_shift(anchor, state.scroll, &state.elements),
        None => anchor_shift(anchor, DVec2::ZERO, &HashMap::new()),
    };
    css * placement.canvas_per_css()
}

/// The part of the rect `entity` is seen at that its page lets through, for
/// hit-testing. `None` when it is hidden or wholly clipped.
pub fn hittable_rect(app: &App, entity: &Entity) -> Option<Rect> {
    let seen = seen(app, entity)?;
    match seen.clip {
        Some(page) => clipped(seen.entity.rect, page),
        None => Some(seen.entity.rect),
    }
}

/// The part of `rect` inside `clip`. A rect with no area is kept whole when
/// it lies inside.
fn clipped(rect: Rect, clip: Rect) -> Option<Rect> {
    let low = geometry::origin(rect).max(geometry::origin(clip));
    let high = (geometry::origin(rect) + geometry::size(rect))
        .min(geometry::origin(clip) + geometry::size(clip));
    (high.x >= low.x && high.y >= low.y).then(|| geometry::rect(low, high - low))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touching_the_edge_is_not_leaving() {
        let page = Rect::new(100.0, 100.0, 400.0, 300.0);
        let rows = [
            (Rect::new(100.0, 400.0, 50.0, 50.0), false),
            (Rect::new(100.0, 401.0, 50.0, 50.0), true),
            (Rect::new(40.0, 100.0, 50.0, 50.0), true),
        ];
        for (rect, left) in rows {
            assert_eq!(left_page(page, rect), left, "{rect:?}");
        }
    }

    #[test]
    fn a_rect_is_clipped_to_its_page() {
        let page = Rect::new(100.0, 100.0, 400.0, 300.0);
        let rows = [
            (
                Rect::new(90.0, 380.0, 50.0, 50.0),
                Some(Rect::new(100.0, 380.0, 40.0, 20.0)),
            ),
            (Rect::new(90.0, 410.0, 50.0, 50.0), None),
        ];
        for (rect, want) in rows {
            assert_eq!(clipped(rect, page), want, "{rect:?}");
        }
    }
}
