//! Whether the zoom is still moving: one answer for everything drawn at a
//! resolution chosen for the zoom.
//!
//! Canvas text's glyph raster, an svg's raster and a page's texture tier are
//! each made for one zoom. Redoing them on every step of a pinch costs more
//! than a frame has, so each is stretched while the zoom moves and redone
//! once it has held still for [`ZOOM_SETTLE_MS`].
//!
//! A frame that happens to come between two steps of a gesture, as the
//! display's refresh, a page's paint or a gif's next frame make one, is not
//! a zoom that has stopped: the zoom has to hold for a while, not for one
//! look.

/// How long the zoom holds still before what was made for an older zoom is
/// made again for this one.
pub const ZOOM_SETTLE_MS: u64 = 120;

/// The zoom last seen and when it last changed, on the session's clock.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ZoomSettle {
    zoom: Option<f32>,
    moved_ms: Option<u64>,
}

impl ZoomSettle {
    /// Notes the zoom at `now_ms`. The first zoom seen is where it starts,
    /// not a move.
    pub(crate) fn observe(&mut self, zoom: f32, now_ms: u64) {
        match self.zoom {
            Some(seen) if seen.to_bits() == zoom.to_bits() => {}
            Some(_) => {
                self.zoom = Some(zoom);
                self.moved_ms = Some(now_ms);
            }
            None => self.zoom = Some(zoom),
        }
    }

    /// Whether the zoom changed less than [`ZOOM_SETTLE_MS`] before `now_ms`.
    pub(crate) fn moving(&self, now_ms: u64) -> bool {
        self.moved_ms
            .is_some_and(|moved| now_ms.saturating_sub(moved) < ZOOM_SETTLE_MS)
    }
}

impl crate::App {
    /// Whether the zoom is still moving: it changed less than
    /// [`ZOOM_SETTLE_MS`] ago by the clock of the latest tick. While it is,
    /// what is drawn at a zoom's resolution is stretched and not redone.
    pub fn zoom_moving(&self) -> bool {
        self.session.zoom_settle.moving(self.session.now_ms)
    }
}
