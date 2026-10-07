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
    fn a_still_camera_rasterises_at_its_own_zoom() {
        let mut hold = RasterHold::default();
        assert_eq!(
            [hold.raster_zoom(1.0, false), hold.raster_zoom(1.1, false)],
            [1.0, 1.1]
        );
    }

    #[test]
    fn zooming_holds_the_size_from_before_the_gesture() {
        let mut hold = RasterHold::default();
        hold.raster_zoom(1.0, false);
        assert_eq!(
            [hold.raster_zoom(1.05, true), hold.raster_zoom(1.2, true)],
            [1.0, 1.0]
        );
    }

    #[test]
    fn settling_refreshes_to_the_final_zoom() {
        let mut hold = RasterHold::default();
        hold.raster_zoom(1.0, false);
        hold.raster_zoom(1.2, true);
        assert_eq!(hold.raster_zoom(1.2, false), 1.2);
    }

    #[test]
    fn a_long_zoom_refreshes_when_the_stretch_leaves_the_band() {
        let mut hold = RasterHold::default();
        hold.raster_zoom(1.0, false);
        // 1.3 is past the band, so it becomes the new held size; 1.5 is
        // then within the band of 1.3.
        assert_eq!(
            [hold.raster_zoom(1.3, true), hold.raster_zoom(1.5, true)],
            [1.3, 1.3]
        );
    }

    #[test]
    fn zooming_out_refreshes_below_the_band_too() {
        let mut hold = RasterHold::default();
        hold.raster_zoom(1.0, false);
        assert_eq!(
            [hold.raster_zoom(0.8, true), hold.raster_zoom(0.7, true)],
            [1.0, 0.7]
        );
    }

    #[test]
    fn the_first_frame_of_a_gesture_with_nothing_held_uses_its_zoom() {
        assert_eq!(RasterHold::default().raster_zoom(0.4, true), 0.4);
    }
}
