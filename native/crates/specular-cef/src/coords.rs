//! Coordinate mapping between page CSS space and texels.
//!
//! Two spaces meet at the CEF boundary (the app maps canvas points into page
//! CSS before calling in):
//!
//! - **Page CSS** (CEF "view"/DIP coordinates): what `SendMouse*Event`,
//!   `GetViewRect`, `OnPopupSize` and IME character bounds use.
//! - **Texels**: frame pixels, CSS × device scale factor; what `OnPaint`
//!   dirty rects, frame sizes and [`specular_core::FrameLayer::Popup`] use.

use specular_core::{PixelRect, PixelSize};

/// Builds a [`PixelRect`] from CEF's signed `cef_rect_t` fields, treating a
/// negative extent as empty.
pub fn rect_from_cef(x: i32, y: i32, width: i32, height: i32) -> PixelRect {
    PixelRect::new(x, y, width.max(0) as u32, height.max(0) as u32)
}

/// Scales a CSS-space rect to the smallest texel rect covering it.
pub fn css_rect_to_texels(rect: PixelRect, scale: f32) -> PixelRect {
    let left = (rect.x as f32 * scale).floor();
    let top = (rect.y as f32 * scale).floor();
    let right = ((rect.x as f32 + rect.width as f32) * scale).ceil();
    let bottom = ((rect.y as f32 + rect.height as f32) * scale).ceil();
    PixelRect::new(
        left as i32,
        top as i32,
        (right - left).max(0.0) as u32,
        (bottom - top).max(0.0) as u32,
    )
}

/// Places a popup widget (CEF `OnPopupSize`, CSS) in the view's texel space.
///
/// Chromium keeps popups inside the screen rect we report, which is the view
/// rect, so this is a safety net: a popup that would overhang is shifted
/// back inside, and one larger than the view is pinned to its origin (the
/// compositor clips it). Its size is never changed, because it must keep
/// matching the `PET_POPUP` frames painted for it.
pub fn place_popup(css: PixelRect, scale: f32, view: PixelSize) -> PixelRect {
    let texels = css_rect_to_texels(css, scale);
    let fit = |pos: i32, len: u32, bound: u32| -> i32 {
        if len >= bound {
            0
        } else {
            pos.clamp(0, (bound - len) as i32)
        }
    };
    PixelRect::new(
        fit(texels.x, texels.width, view.width),
        fit(texels.y, texels.height, view.height),
        texels.width,
        texels.height,
    )
}

/// Union of rects, e.g. IME character bounds into one composition box.
/// Zero-width rects (a caret) still contribute their position.
pub fn union_rects(rects: impl IntoIterator<Item = PixelRect>) -> Option<PixelRect> {
    rects
        .into_iter()
        .map(|r| {
            (
                i64::from(r.x),
                i64::from(r.y),
                i64::from(r.x) + i64::from(r.width),
                i64::from(r.y) + i64::from(r.height),
            )
        })
        .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
        .map(|(left, top, right, bottom)| {
            PixelRect::new(
                left as i32,
                top as i32,
                (right - left) as u32,
                (bottom - top) as u32,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_scale_rounds_texel_rect_outward() {
        assert_eq!(
            css_rect_to_texels(PixelRect::new(1, 1, 3, 3), 1.5),
            PixelRect::new(1, 1, 5, 5)
        );
    }

    #[test]
    fn popup_inside_view_keeps_its_scaled_position() {
        let placed = place_popup(
            PixelRect::new(10, 20, 100, 50),
            2.0,
            PixelSize::new(800, 600),
        );
        assert_eq!(placed, PixelRect::new(20, 40, 200, 100));
    }

    #[test]
    fn popup_overhanging_bottom_right_is_shifted_inside() {
        let placed = place_popup(
            PixelRect::new(350, 280, 100, 50),
            1.0,
            PixelSize::new(400, 300),
        );
        assert_eq!(placed, PixelRect::new(300, 250, 100, 50));
    }

    #[test]
    fn popup_larger_than_view_is_pinned_to_origin_without_resizing() {
        let placed = place_popup(
            PixelRect::new(-5, 10, 500, 20),
            1.0,
            PixelSize::new(400, 300),
        );
        assert_eq!(placed, PixelRect::new(0, 10, 500, 20));
    }

    #[test]
    fn union_covers_all_character_bounds() {
        let union = union_rects([PixelRect::new(10, 10, 5, 12), PixelRect::new(30, 8, 6, 12)]);
        assert_eq!(union, Some(PixelRect::new(10, 8, 26, 14)));
    }
}
