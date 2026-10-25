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
