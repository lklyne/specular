//! Where a frame's time on the CPU went, for the bench.

use std::time::Duration;

/// The time [`render_scene`](crate::Compositor::render_scene) spent in each
/// step of one frame. The steps do not overlap, so their sum is the frame's
/// CPU time in the compositor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StageTimes {
    /// Resolving items against the camera and culling, without the shaping
    /// it asked for.
    pub cull: Duration,
    /// Shaping text no cached buffer matched.
    pub shaping: Duration,
    /// Grouping the visible items into draws.
    pub batching: Duration,
    /// Tessellating polygons and paths.
    pub tessellation: Duration,
    /// Building instances and the draw list, without tessellation.
    pub build: Duration,
    /// Laying shaped text out as glyph quads, rasterising new glyphs and
    /// uploading both.
    pub glyphs: Duration,
    /// Writing the instance, vertex and index buffers.
    pub upload: Duration,
    /// Encoding the pass and submitting it.
    pub submit: Duration,
}

impl StageTimes {
    /// Every step added up.
    pub fn total(&self) -> Duration {
        self.cull
            + self.shaping
            + self.batching
            + self.tessellation
            + self.build
            + self.glyphs
            + self.upload
            + self.submit
    }
}
