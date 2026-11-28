//! A page's texture scale stays inside the largest texture a GPU makes.

use specular_core::{CssSize, MAX_TEXTURE_SIDE};

#[test]
fn a_tall_page_rasters_coarser_to_fit_a_texture() {
    for (viewport, asked, want) in [
        (CssSize::new(1200, 800), 2.0, 2.0),
        (CssSize::new(1200, 8192), 2.0, 2.0),
        (CssSize::new(1200, 9660), 2.0, 16_384.0 / 9660.0),
        (CssSize::new(40_000, 300), 1.0, 16_384.0 / 40_000.0),
        (CssSize::new(0, 0), 2.0, 2.0),
    ] {
        let scale = viewport.scale_within(asked, MAX_TEXTURE_SIDE);
        let frame = viewport.to_pixels(scale);
        assert!(
            frame.width <= MAX_TEXTURE_SIDE && frame.height <= MAX_TEXTURE_SIDE,
            "{viewport:?} at {asked} gave {frame:?}"
        );
        assert!(
            (scale - want).abs() < 1e-3,
            "{viewport:?} at {asked}: {scale}"
        );
    }
}
