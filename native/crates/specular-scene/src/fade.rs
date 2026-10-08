//! [`PageBand`]: how something that follows a page's scroll shows through
//! that page. It is drawn whole over the page, cut off at the page's sides,
//! and fades out over a short reach above and below it, so an item scrolling
//! out of its page thins away at the edge and does not vanish at a line.
//!
//! No pipeline has a per-pixel alpha mask, and glyphs are drawn by a
//! renderer that takes none. So the fade is steps: the reach is cut into
//! strips, and the item is drawn again in each one it touches, clipped to
//! the strip and at the strip's share of its opacity. Every draw honours a
//! clip and an opacity, so every kind fades the same way.

use crate::{Item, Rect};

/// How many strips the fade is cut into, each way.
const STEPS: u16 = 16;

/// A page and how far past its top and bottom what follows it still shows,
/// in one item space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageBand {
    /// The page's rect.
    pub page: Rect,
    /// The height of the fade above the page and below it.
    pub reach: f32,
}

impl PageBand {
    /// Pushes `item` as it shows through the band, in paint order: its
    /// faded strips, then the part over the page at its own opacity.
    /// `extent` is where the item may paint when its draw cannot say (text),
    /// so strips it cannot touch are left out.
    pub fn through(self, item: Item, extent: Rect, out: &mut Vec<Item>) {
        let extent = item.draw.bounds().unwrap_or(extent);
        let inside = |rect: Rect| {
            if !rect.intersects(extent) {
                return None;
            }
            match item.clip {
                Some(own) => own.intersection(rect),
                None => Some(rect),
            }
        };
        let page = self.page;
        let step = self.reach / f32::from(STEPS);
        for index in 0..STEPS {
            // The middle of each strip sets its share, so the steps straddle
            // the straight line from all to nothing.
            let share = 1.0 - (f32::from(index) + 0.5) / f32::from(STEPS);
            let (above, below) = (
                page.y - f32::from(index + 1) * step,
                page.bottom() + f32::from(index) * step,
            );
            for top in [above, below] {
                if let Some(clip) = inside(Rect::new(page.x, top, page.width, step)) {
                    out.push(Item {
                        clip: Some(clip),
                        opacity: item.opacity * share,
                        ..item.clone()
                    });
                }
            }
        }
        if let Some(clip) = inside(page) {
            out.push(Item {
                clip: Some(clip),
                ..item
            });
        }
    }
}
