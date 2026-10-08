//! Scroll-follow: an entity hooked to a page tracks the document content it
//! was placed over. Its anchor records the page's scroll at placement, and
//! it is drawn and hit shifted by how far the page has scrolled since, so a
//! sticky stays on the paragraph it was put beside.
//!
//! The stored rect stays the truth. The shift is folded into it when the
//! entity is moved or re-anchored (see [`fold`]), which restamps the scroll.
//! An anchor with no scroll is pinned to the page's frame and never shifts.
//!
//! Drawing, hit-testing, outlines and marquees all read the shifted rect from
//! here, so what is seen is what is grabbed.

use std::borrow::Cow;
use std::collections::HashMap;

use glam::DVec2;
use specular_doc::{Drawing, Entity, EntityId, Kind, PageAnchor, Rect};

use crate::{App, geometry, strokes};

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

/// How far an item whose anchor recorded `recorded` has moved, in canvas
/// units, with its page at `live`: subtract it from the stored position.
/// `per_css` is the page's canvas units per CSS pixel.
pub fn shift_of(recorded: DVec2, live: DVec2, per_css: DVec2) -> DVec2 {
    (live - recorded) * per_css
}

/// Whether `shown` has left `page` altogether. A rect that only touches the
/// page's edge is still there.
pub fn left_page(page: Rect, shown: Rect) -> bool {
    shown.x > page.x + page.width
        || shown.y > page.y + page.height
        || shown.x + shown.width < page.x
        || shown.y + shown.height < page.y
}

/// A page's scroll, and the scale of its CSS pixels, as an anchor reads
/// them. A snapshot, so a step that edits the document can read it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PageScroll {
    live: DVec2,
    per_css: DVec2,
}

/// The scroll of every page, as the app has it now.
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
        let (Some(recorded), Some(page)) = (recorded_scroll(anchor), self.0.get(&anchor.page_id))
        else {
            return DVec2::ZERO;
        };
        shift_of(recorded, page.live, page.per_css)
    }
}

/// The anchor `anchor` becomes when its entity's shift is folded into its
/// stored position: the same page, restamped at the page's scroll now. An
/// anchor that records no scroll stays as it is.
pub(crate) fn restamped(scrolls: &Scrolls, anchor: &PageAnchor) -> PageAnchor {
    if recorded_scroll(anchor).is_none() {
        return anchor.clone();
    }
    let live = scrolls.live(&anchor.page_id);
    PageAnchor {
        scroll_x: Some(live.x),
        scroll_y: Some(live.y),
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

/// `entity` as it is seen: shifted by its page's scroll, and clipped to the
/// page while it is. `None` when it has scrolled out of the page altogether,
/// which hides it and takes it out of hit-testing.
pub fn seen<'a>(app: &App, entity: &'a Entity) -> Option<Seen<'a>> {
    let shift = shift_for(app, entity);
    if shift == DVec2::ZERO {
        return Some(Seen {
            entity: Cow::Borrowed(entity),
            clip: None,
        });
    }
    let page = app.page_placement(&entity.anchor.as_ref()?.page_id)?.rect;
    let shown = shifted(entity, -shift);
    (!left_page(page, shown.rect)).then_some(Seen {
        entity: Cow::Owned(shown),
        clip: Some(page),
    })
}

/// The rect `entity` is seen at, or `None` when it is hidden.
pub fn shown_rect(app: &App, entity: &Entity) -> Option<Rect> {
    seen(app, entity).map(|seen| seen.entity.rect)
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

fn shift_for(app: &App, entity: &Entity) -> DVec2 {
    let Some(anchor) = &entity.anchor else {
        return DVec2::ZERO;
    };
    let (Some(recorded), Some(placement)) =
        (recorded_scroll(anchor), app.page_placement(&anchor.page_id))
    else {
        return DVec2::ZERO;
    };
    shift_of(
        recorded,
        app.page_scroll(&anchor.page_id),
        placement.canvas_per_css(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_scrolled_down_moves_its_content_up() {
        let shift = shift_of(
            DVec2::new(0.0, 100.0),
            DVec2::new(0.0, 340.0),
            DVec2::new(1.0, 1.0),
        );
        assert_eq!(shift, DVec2::new(0.0, 240.0));

        {
            // A 1280px page drawn 640 wide: a CSS pixel is half a unit.
            let shift = shift_of(DVec2::ZERO, DVec2::new(0.0, 200.0), DVec2::new(0.5, 0.5));
            assert_eq!(shift, DVec2::new(0.0, 100.0));
        }
    }

    #[test]
    fn touching_the_edge_is_not_leaving() {
        let page = Rect::new(100.0, 100.0, 400.0, 300.0);
        assert!(!left_page(page, Rect::new(100.0, 400.0, 50.0, 50.0)));
        assert!(left_page(page, Rect::new(100.0, 401.0, 50.0, 50.0)));
        assert!(left_page(page, Rect::new(40.0, 100.0, 50.0, 50.0)));
    }

    #[test]
    fn a_rect_is_clipped_to_its_page() {
        let page = Rect::new(100.0, 100.0, 400.0, 300.0);
        assert_eq!(
            clipped(Rect::new(90.0, 380.0, 50.0, 50.0), page),
            Some(Rect::new(100.0, 380.0, 40.0, 20.0))
        );
        assert_eq!(clipped(Rect::new(90.0, 410.0, 50.0, 50.0), page), None);
    }
}
