//! Which zoom canvas text is rasterised at.
//!
//! Each new zoom is a new glyph size, so a zoom gesture that rasterises at
//! every step re-fills the glyph atlas every frame (ADR 0039). While the
//! camera is zooming the glyphs are held at the size they had and the quads
//! are stretched instead; the first frame after the gesture rasterises at
//! the real size again.

/// How far the camera may zoom out of a held raster before it is refreshed
/// anyway. Below this the glyphs are being shrunk to under three quarters.
const MIN_STRETCH: f32 = 0.75;
/// How far the camera may zoom into a held raster before it is refreshed
/// anyway. Past this the stretched glyphs are visibly soft.
const MAX_STRETCH: f32 = 1.25;

/// The zoom the glyph atlas is currently rasterised for.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct RasterHold {
    held: Option<f32>,
}

impl RasterHold {
    /// The zoom to rasterise canvas text at this frame. The quads are then
    /// stretched by `zoom / raster_zoom`.
    pub(crate) fn raster_zoom(&mut self, zoom: f32, zooming: bool) -> f32 {
        if zooming
            && let Some(held) = self.held
            && (MIN_STRETCH..=MAX_STRETCH).contains(&(zoom / held))
        {
            return held;
        }
        self.held = Some(zoom);
        zoom
    }

    /// Drops the held raster, so this frame's zoom is used as it is.
    pub(crate) fn release(&mut self, zoom: f32) {
        self.held = Some(zoom);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zoom_gesture_holds_the_raster_until_it_settles_or_leaves_the_band() {
        // Each step is (zoom, zooming) and the raster zoom it should give.
        for (name, steps) in [
            ("still", vec![(1.0, false, 1.0), (1.1, false, 1.1)]),
            (
                "zooming holds",
                vec![(1.0, false, 1.0), (1.05, true, 1.0), (1.2, true, 1.0)],
            ),
            (
                "settling refreshes",
                vec![(1.0, false, 1.0), (1.2, true, 1.0), (1.2, false, 1.2)],
            ),
            // 1.3 is past the band, so it becomes the new held size; 1.5 is
            // then within the band of 1.3.
            (
                "zooming in past the band",
                vec![(1.0, false, 1.0), (1.3, true, 1.3), (1.5, true, 1.3)],
            ),
            (
                "zooming out past the band",
                vec![(1.0, false, 1.0), (0.8, true, 1.0), (0.7, true, 0.7)],
            ),
            ("nothing held", vec![(0.4, true, 0.4)]),
        ] {
            let mut hold = RasterHold::default();
            for (zoom, zooming, expected) in steps {
                assert_eq!(
                    hold.raster_zoom(zoom, zooming),
                    expected,
                    "{name} at {zoom}"
                );
            }
        }
    }
}
