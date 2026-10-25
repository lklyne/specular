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
