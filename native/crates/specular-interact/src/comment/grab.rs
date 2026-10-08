//! Which page a comment region belongs to. A region drawn over pages is
//! theirs only if it grabbed something in one, which the shell has to ask
//! the pages.

use specular_doc::{EntityId, Rect};

use super::{create, draft};
use crate::{App, Effect, geometry};

/// A page a comment region lies over, and the part of the page it covers.
#[derive(Debug, Clone, PartialEq)]
pub struct PageRegion {
    /// The page entity.
    pub page: EntityId,
    /// What the region covers of the page, in its viewport CSS pixels.
    pub rect: Rect,
}

/// What a comment region grabbed in one page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageGrab {
    /// The page entity.
    pub page: EntityId,
    /// How many of the page's elements lie in the region.
    pub elements: usize,
}

/// The page a region with these `grabs` is bound to: the first that grabbed
/// an element. A page the region merely lies over is in the list with none,
/// so being listed is not a grab, and a region that grabbed nothing stays on
/// the canvas.
pub(crate) fn grabbing_page(grabs: &[PageGrab]) -> Option<&EntityId> {
    grabs
        .iter()
        .find(|grab| grab.elements > 0)
        .map(|grab| &grab.page)
}

/// The pages the canvas-space `region` lies over, front to back.
pub(super) fn pages_under(app: &App, region: Rect) -> Vec<PageRegion> {
    let mut pages: Vec<PageRegion> = app
        .pages()
        .filter_map(|(id, _, placement)| {
            let covered = geometry::intersection(region, placement.rect)?;
            Some(PageRegion {
                page: id.clone(),
                rect: super::canvas_in_css(placement, covered),
            })
        })
        .collect();
    pages.reverse();
    pages
}

/// The shell said what `region` grabbed. The region becomes a draft in the
/// page it grabbed from, or on the canvas when it grabbed nothing.
///
/// An answer that finds a drag in flight is dropped, as is one whose page
/// has gone: what the user is doing now is not to be interrupted, and a
/// region cannot be bound to nothing.
pub(crate) fn on_region_grab(
    app: &mut App,
    region: Rect,
    grabs: &[PageGrab],
    effects: &mut Vec<Effect>,
) {
    if app.session.gesture.is_some() {
        return;
    }
    let made = match grabbing_page(grabs) {
        Some(page) => create::page_region(app, page, region),
        None => Some(create::canvas_region(app, region)),
    };
    if let Some(made) = made {
        draft::open(app, made, effects);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grab(page: &str, elements: usize) -> PageGrab {
        PageGrab {
            page: EntityId::from(page),
            elements,
        }
    }

    fn bound(grabs: &[PageGrab]) -> Option<&str> {
        grabbing_page(grabs).map(EntityId::as_str)
    }

    #[test]
    fn the_first_page_that_grabbed_an_element_wins() {
        assert_eq!(
            bound(&[grab("p1", 0), grab("p2", 3), grab("p3", 5)]),
            Some("p2")
        );

        {
            assert_eq!(bound(&[grab("p1", 0), grab("p2", 0)]), None);
        }

        {
            assert_eq!(bound(&[]), None);
        }
    }
}
