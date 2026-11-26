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

/// How long an input may go unanswered by a paint before the next input
/// gives up on it. A key's down, character and up are sent together, well
/// inside it; a click the page shows no change for is not, and must not be
/// resolved by the paint of a key pressed a second later.
const UNANSWERED: Duration = Duration::from_millis(100);

/// Tracks forwarded input until the target page's repaint is presented.
#[derive(Debug, Clone, Default)]
pub(crate) struct InputLatencyProbe {
    tracker: LatencyTracker,
    /// Latest input forwarded and not yet answered by a paint, and when the
    /// oldest input still unanswered was sent.
    awaiting_paint: Option<(PageId, InputSeq, Instant)>,
    /// Latest input whose page has painted; resolved at the next present.
    awaiting_present: Option<InputSeq>,
}

impl InputLatencyProbe {
    /// Notes that input was forwarded to `page` at `sent`.
    pub(crate) fn input_sent(&mut self, page: PageId, sent: Instant) {
        let mut since = sent;
        if let Some((_, unanswered, at)) = self.awaiting_paint {
            if sent.saturating_duration_since(at) > UNANSWERED {
                self.tracker.abandon_through(unanswered);
            } else {
                since = at;
            }
        }
        let seq = self.tracker.event(sent);
        self.awaiting_paint = Some((page, seq, since));
    }

    /// Watches drained page events for the awaited page's next frame.
    pub(crate) fn observe(&mut self, event: &PageEvent) {
        if let (Some((page, seq, _)), PageEvent::Frame(frame)) = (self.awaiting_paint, event)
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

#[cfg(test)]
mod tests {
    use specular_core::{CpuFrame, CssSize, FrameEvent, FrameLayer, PageFrame, PixelSize};

    use super::*;

    fn painted(page: PageId, at: Instant) -> PageEvent {
        PageEvent::Frame(FrameEvent {
            page,
            layer: FrameLayer::View,
            viewport: CssSize::new(10, 10),
            frame: PageFrame::Cpu(CpuFrame {
                size: PixelSize::new(1, 1),
                stride: 4,
                bgra: vec![0; 4],
                dirty: Vec::new(),
            }),
            produced_at: at,
        })
    }

    #[test]
    fn a_click_the_page_never_answered_is_not_resolved_by_a_later_keys_paint() {
        let page = PageId(1);
        let start = Instant::now();
        let ms = |n: u64| start + Duration::from_millis(n);
        let mut probe = InputLatencyProbe::default();
        // A click (down and up) that changes nothing on the page.
        probe.input_sent(page, ms(0));
        probe.input_sent(page, ms(1));
        // A second later, a key's down, character and up, which it paints.
        for at in [1_000, 1_001, 1_002] {
            probe.input_sent(page, ms(at));
        }
        probe.observe(&painted(page, ms(1_005)));
        probe.presented(ms(1_010));
        let summary = probe.summary();
        assert_eq!((summary.samples, summary.unresolved), (3, 2));
        assert!(summary.max_ms <= 10.0, "{summary:?}");
    }
}
