//! Input-to-present latency: time from forwarding an input event to a page
//! until a frame showing that page's next repaint reaches the screen.
//!
//! The bench crate's [`LatencyTracker`] does the bookkeeping; this adapter
//! decides what "reflects the input" means for forwarded page input: the
//! first frame the page paints after the event. Pages also paint for their
//! own reasons (animation), so a sample is a lower bound on how long the
//! page took to respond, never an overestimate of the shell's share.

use std::time::{Duration, Instant};

use specular_bench::{InputSeq, LatencySummary, LatencyTracker};
use specular_core::{PageEvent, PageId};

/// Tracks forwarded input until the target page's repaint is presented.
#[derive(Debug, Clone, Default)]
pub(crate) struct InputLatencyProbe {
    tracker: LatencyTracker,
    /// Latest input forwarded and not yet answered by a paint.
    awaiting_paint: Option<(PageId, InputSeq)>,
    /// Latest input whose page has painted; resolved at the next present.
    awaiting_present: Option<InputSeq>,
}

impl InputLatencyProbe {
    /// Notes that input was forwarded to `page` at `sent`.
    pub(crate) fn input_sent(&mut self, page: PageId, sent: Instant) {
        let seq = self.tracker.event(sent);
        self.awaiting_paint = Some((page, seq));
    }

    /// Watches drained page events for the awaited page's next frame.
    pub(crate) fn observe(&mut self, event: &PageEvent) {
        if let (Some((page, seq)), PageEvent::Frame(frame)) = (self.awaiting_paint, event)
            && frame.page == page
        {
            self.awaiting_paint = None;
            self.awaiting_present = Some(seq);
        }
    }

    /// Call after presenting at `presented`; yields the newest latency
    /// resolved by this frame, if any.
    pub(crate) fn presented(&mut self, presented: Instant) -> Option<Duration> {
        let seq = self.awaiting_present.take()?;
        if self.tracker.presented(seq, presented) == 0 {
            return None;
        }
        self.tracker.samples().last().copied()
    }

    /// Distribution of every sample so far.
    pub(crate) fn summary(&self) -> LatencySummary {
        self.tracker.summary()
    }
}
