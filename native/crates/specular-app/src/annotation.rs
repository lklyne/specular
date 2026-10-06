//! Comment annotations: a region with a pin, bound to a page or the canvas.

use specular_core::{CanvasRect, PageId};

use crate::placement::PlacedPage;

/// What a region's rect is measured against.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Anchor {
    /// Fixed canvas units; stays put when pages move.
    Canvas(CanvasRect),
    /// A rect in the unit square of a page (`0..1` on each axis), so the
    /// annotation travels with the page and scales when it is resized.
    Page { page: PageId, fraction: CanvasRect },
}

/// A commented region. The pin is drawn at the region's top-left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Annotation {
    pub(crate) anchor: Anchor,
}

impl Annotation {
    /// An annotation fixed to the canvas at `region`.
    pub(crate) const fn canvas_bound(region: CanvasRect) -> Self {
        Self {
            anchor: Anchor::Canvas(region),
        }
    }

    /// An annotation of canvas-space `region` that follows `page`, stored
    /// relative to the page's rect at the moment of creation.
    pub(crate) fn page_bound(page: &PlacedPage, region: CanvasRect) -> Self {
        let size = page.rect.size().max(glam::Vec2::splat(f32::EPSILON));
        let origin = (region.origin() - page.rect.origin()) / size;
        let extent = region.size() / size;
        Self {
            anchor: Anchor::Page {
                page: page.page,
                fraction: CanvasRect::new(origin.x, origin.y, extent.x, extent.y),
            },
        }
    }

    /// The region in canvas space, or `None` when its page is gone.
    pub(crate) fn rect(&self, placed: &[PlacedPage]) -> Option<CanvasRect> {
        match self.anchor {
            Anchor::Canvas(rect) => Some(rect),
            Anchor::Page { page, fraction } => {
                let rect = placed.iter().find(|p| p.page == page)?.rect;
                Some(CanvasRect::new(
                    rect.x + fraction.x * rect.width,
                    rect.y + fraction.y * rect.height,
                    fraction.width * rect.width,
                    fraction.height * rect.height,
                ))
            }
        }
    }
}

/// `count` page-bound annotations spread over `placed`, round-robin, with
/// position and size varied by index arithmetic alone so a benchmark draws
/// the same UI every run.
pub(crate) fn seed(count: usize, placed: &[PlacedPage]) -> Vec<Annotation> {
    if placed.is_empty() {
        return Vec::new();
    }
    (0..count)
        .map(|index| {
            let page = &placed[index % placed.len()];
            // Offsets top out at 0.77 / 0.68 and sizes at 0.20 / 0.14, so
            // every region stays inside its page.
            let x = 0.05 + 0.08 * ((index * 7) % 10) as f32;
            let y = 0.05 + 0.07 * ((index * 3 + 1) % 10) as f32;
            let width = 0.12 + 0.02 * (index % 5) as f32;
            let height = 0.08 + 0.02 * ((index / 2) % 4) as f32;
            Annotation {
                anchor: Anchor::Page {
                    page: page.page,
                    fraction: CanvasRect::new(x, y, width, height),
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use specular_core::CssSize;

    use super::*;

    /// Fractions round-trip through `f32`, so compare to a small tolerance.
    fn assert_rect_close(actual: Option<CanvasRect>, expected: CanvasRect) {
        let actual = actual.expect("annotation has a rect");
        let off = (actual.origin() - expected.origin())
            .abs()
            .max((actual.size() - expected.size()).abs());
        assert!(off.max_element() < 1e-3, "{actual:?} vs {expected:?}");
    }

    fn page(id: u64, rect: CanvasRect) -> PlacedPage {
        PlacedPage::new(PageId(id), rect, CssSize::new(100, 100))
    }

    #[test]
    fn page_bound_region_follows_a_moved_page() {
        let mut pages = [page(1, CanvasRect::new(0.0, 0.0, 200.0, 100.0))];
        let note = Annotation::page_bound(&pages[0], CanvasRect::new(20.0, 10.0, 40.0, 30.0));
        pages[0].rect = CanvasRect::new(500.0, 300.0, 200.0, 100.0);
        assert_rect_close(note.rect(&pages), CanvasRect::new(520.0, 310.0, 40.0, 30.0));
    }

    #[test]
    fn page_bound_region_scales_with_a_resized_page() {
        let mut pages = [page(1, CanvasRect::new(0.0, 0.0, 200.0, 100.0))];
        let note = Annotation::page_bound(&pages[0], CanvasRect::new(20.0, 10.0, 40.0, 30.0));
        pages[0].rect = CanvasRect::new(0.0, 0.0, 400.0, 200.0);
        assert_rect_close(note.rect(&pages), CanvasRect::new(40.0, 20.0, 80.0, 60.0));
    }

    #[test]
    fn canvas_bound_region_ignores_pages() {
        let region = CanvasRect::new(5.0, 5.0, 10.0, 10.0);
        assert_eq!(Annotation::canvas_bound(region).rect(&[]), Some(region));
    }

    #[test]
    fn page_bound_region_without_its_page_has_no_rect() {
        let pages = [page(1, CanvasRect::new(0.0, 0.0, 10.0, 10.0))];
        let note = Annotation::page_bound(&pages[0], CanvasRect::new(1.0, 1.0, 2.0, 2.0));
        assert_eq!(note.rect(&[]), None);
    }

    #[test]
    fn seed_spreads_round_robin_over_pages() {
        let pages = [
            page(1, CanvasRect::new(0.0, 0.0, 100.0, 100.0)),
            page(2, CanvasRect::new(200.0, 0.0, 100.0, 100.0)),
        ];
        let owners: Vec<_> = seed(5, &pages)
            .iter()
            .map(|note| match note.anchor {
                Anchor::Page { page, .. } => page.0,
                Anchor::Canvas(_) => 0,
            })
            .collect();
        assert_eq!(owners, [1, 2, 1, 2, 1]);
    }

    #[test]
    fn seeded_regions_stay_inside_their_page() {
        let pages = [page(1, CanvasRect::new(100.0, 100.0, 1280.0, 800.0))];
        let outside = seed(200, &pages).iter().any(|note| {
            note.rect(&pages).is_none_or(|region| {
                region.x < 100.0
                    || region.y < 100.0
                    || region.x + region.width > 1380.0
                    || region.y + region.height > 900.0
            })
        });
        assert!(!outside);
    }

    #[test]
    fn seed_is_deterministic_and_varied() {
        let pages = [page(1, CanvasRect::new(0.0, 0.0, 100.0, 100.0))];
        let (a, b) = (seed(6, &pages), seed(6, &pages));
        assert_eq!(a, b);
        assert_ne!(a[0], a[1]);
    }

    #[test]
    fn seed_without_pages_makes_nothing() {
        assert_eq!(seed(4, &[]).len(), 0);
    }
}
