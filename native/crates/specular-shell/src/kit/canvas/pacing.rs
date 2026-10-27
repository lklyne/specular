//! When the canvas reads the models, and when its display link rests.

use std::time::{Duration, Instant};

use super::Canvas;

/// The least time between two readings of the models while the app keeps
/// changing. Building them walks the whole document, every frame of a pan
/// if nothing held it back, and GPUI lays its whole tree out again when one
/// changed, as the toolbar's zoom readout does every frame of a zoom. Both
/// happen on the thread the canvas draws from.
const MODELS_INTERVAL: Duration = Duration::from_millis(100);

/// Holds the reading of the models to one a [`MODELS_INTERVAL`]: the first
/// at once, and whatever changed meanwhile in one reading when the interval
/// is over, so GPUI always ends up showing the app as it is.
#[derive(Debug, Default)]
pub(super) struct ModelGate {
    opened_at: Option<Instant>,
    owed: bool,
}

impl ModelGate {
    /// The app may have changed.
    pub(super) fn ask(&mut self) {
        self.owed = true;
    }

    /// Whether to read the models now. Asked every turn.
    pub(super) fn open(&mut self, now: Instant) -> bool {
        let rested = (self.opened_at).is_none_or(|at| now.duration_since(at) >= MODELS_INTERVAL);
        if !self.owed || !rested {
            return false;
        }
        self.owed = false;
        self.opened_at = Some(now);
        true
    }
}

/// How long after the last owed frame the display link is paused. A page
/// painting a few times a second keeps it running; a canvas nobody touches
/// stops being woken every refresh.
const REST_BEFORE_PAUSE: Duration = Duration::from_secs(1);

/// Whether the display link should be running: it is paused once no frame
/// has been owed for [`REST_BEFORE_PAUSE`].
#[derive(Debug)]
pub(super) struct LinkRest {
    pub(super) owed_at: Instant,
    /// Whether the link is paused now. The shell's timer then turns the
    /// canvas.
    pub(super) paused: bool,
}

impl LinkRest {
    /// A frame is owed, or the app changed and one may be.
    pub(super) fn owed(&mut self, now: Instant) {
        self.owed_at = now;
    }

    fn is_over(&self, now: Instant) -> bool {
        now.duration_since(self.owed_at) >= REST_BEFORE_PAUSE
    }
}

impl Canvas {
    /// Pauses the display link once the canvas has rested a while, and
    /// starts it again when a frame is owed.
    pub(super) fn pace_link(&mut self, now: Instant, wanted: bool) {
        if wanted {
            self.rest.owed(now);
        }
        self.pause_link(self.rest.is_over(now));
    }

    pub(super) fn pause_link(&mut self, paused: bool) {
        if paused == self.rest.paused {
            return;
        }
        self.rest.paused = paused;
        if let Some(surface) = self.runtime.window() {
            surface.native().set_link_paused(paused);
        }
    }

    /// How long the shell's timer waits before it turns the canvas: the
    /// time to the next turn while the link is paused, and `otherwise`
    /// while the link runs and the timer only covers for a window the
    /// system gives no refreshes.
    pub(crate) fn timer_wait(&self, otherwise: Duration) -> Duration {
        match self.rest_until {
            Some(until) if self.rest.paused => {
                (until.saturating_duration_since(Instant::now())).min(otherwise)
            }
            _ => otherwise,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_display_link_is_paused_a_second_after_the_last_owed_frame_and_no_sooner() {
        let start = Instant::now();
        let mut rest = LinkRest {
            owed_at: start,
            paused: false,
        };
        // A page painting twice a second keeps the link running.
        for half_seconds in 1..=4_u32 {
            let now = start + Duration::from_millis(500) * half_seconds;
            assert!(!rest.is_over(now));
            rest.owed(now);
        }
        let last = start + Duration::from_secs(2);
        assert!(!rest.is_over(last + Duration::from_millis(999)));
        assert!(rest.is_over(last + REST_BEFORE_PAUSE));
        rest.owed(last + Duration::from_secs(5));
        assert!(!rest.is_over(last + Duration::from_secs(5)));
    }
}
