//! A decoded image in a file entity's rect, fitted as CSS `object-fit` fits
//! an `<img>` (`ImageInlineRenderer.tsx`).

use specular_doc::ObjectFit;

use crate::{ImageDraw, ImageId, Rect};

/// `image`, `width` by `height` pixels, in `rect`.
///
/// - `Contain` letterboxes: the whole image, as large as fits, centred.
/// - `Cover` crops: the rect is filled and the overflow is cut evenly from
///   both sides.
/// - `Fill` stretches the whole image over the rect.
pub(crate) fn fitted(
    image: ImageId,
    rect: Rect,
    width: u32,
    height: u32,
    fit: ObjectFit,
) -> ImageDraw {
    let (width, height) = (width as f32, height as f32);
    let whole = ImageDraw::new(image, rect);
    if width <= 0.0 || height <= 0.0 || rect.width <= 0.0 || rect.height <= 0.0 {
        return whole;
    }
    let (across, down) = (rect.width / width, rect.height / height);
    match fit {
        ObjectFit::Fill => whole,
        ObjectFit::Contain => {
            let scale = across.min(down);
            let (shown_width, shown_height) = (width * scale, height * scale);
            let shown = Rect::new(
                rect.x + (rect.width - shown_width) / 2.0,
                rect.y + (rect.height - shown_height) / 2.0,
                shown_width,
                shown_height,
            );
            ImageDraw::new(image, shown)
        }
        ObjectFit::Cover => {
            let scale = across.max(down);
            // The part of the scaled image the rect shows, per axis.
            let (kept_width, kept_height) =
                (rect.width / (width * scale), rect.height / (height * scale));
            ImageDraw {
                source: Rect::new(
                    (1.0 - kept_width) / 2.0,
                    (1.0 - kept_height) / 2.0,
                    kept_width,
                    kept_height,
                ),
                ..whole
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMAGE: ImageId = ImageId(7);
    /// A 200 by 100 box at (10, 20).
    const BOX: Rect = Rect::new(10.0, 20.0, 200.0, 100.0);

    #[test]
    fn contain_letterboxes_a_tall_image_and_centres_it() {
        let draw = fitted(IMAGE, BOX, 50, 100, ObjectFit::Contain);
        assert_eq!(draw.rect, Rect::new(85.0, 20.0, 50.0, 100.0));
        assert_eq!(draw.source, ImageDraw::WHOLE);
        // A wide one is centred down the box instead.
        let wide = fitted(IMAGE, BOX, 400, 100, ObjectFit::Contain);
        assert_eq!(wide.rect, Rect::new(10.0, 45.0, 200.0, 50.0));
        // With no size to fit, the box is filled.
        let empty = fitted(IMAGE, BOX, 0, 100, ObjectFit::Contain);
        assert_eq!(empty.rect, BOX);
    }

    #[test]
    fn cover_fills_the_rect_and_crops_the_overflow_evenly() {
        // A square image over a 2:1 box keeps its middle half, top to bottom.
        let tall = fitted(IMAGE, BOX, 100, 100, ObjectFit::Cover);
        assert_eq!(tall.rect, BOX);
        assert_eq!(tall.source, Rect::new(0.0, 0.25, 1.0, 0.5));
        // A 4:1 image keeps its middle half, left to right.
        let wide = fitted(IMAGE, BOX, 400, 100, ObjectFit::Cover);
        assert_eq!(wide.source, Rect::new(0.25, 0.0, 0.5, 1.0));
    }
}
