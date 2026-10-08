//! Collects one profile's presented frames into a [`PhaseReport`].
//!
//! A runner calls [`PhaseRecorder::presented`] once per `present`, with the
//! compositor's view of that frame, and [`PhaseRecorder::finish`] when the
//! profile's last step has been presented. Intervals are measured between
//! presents, as the lab measured rAF-to-rAF, so the first present of a phase
//! starts the clock and contributes no interval.

use std::time::{Duration, Instant};

use crate::{FrameTimes, PhaseReport, ProfileId, TextureStats};

/// What the compositor reports about one presented frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PresentedFrame {
    /// Visible pages drawn as placeholders for want of a texture.
    pub pages_without_texture: u32,
    /// Pages whose texture came through a CPU upload this frame.
    pub cpu_textures: u32,
    /// View-layer page frames admitted since the previous present.
    pub frames_received: u32,
    /// Popup-layer page frames admitted since the previous present.
    pub popup_frames: u32,
    /// Paints refused at the outstanding-texture cap since the previous
    /// present.
    pub frames_dropped_for_pool_pressure: u32,
    /// Shared textures held now, summed over pages.
    pub outstanding_textures: u32,
    /// Shared textures held now by the page holding the most.
    pub max_outstanding_textures: u32,
}

/// Accumulates one phase.
#[derive(Debug, Clone)]
pub struct PhaseRecorder {
    phase: ProfileId,
    first_present: Option<Instant>,
    last_present: Option<Instant>,
    times: FrameTimes,
    draws_without_texture: u64,
    saw_cpu_texture: bool,
    textures: TextureStats,
}

impl PhaseRecorder {
    /// Starts recording `phase`.
    pub fn new(phase: ProfileId) -> Self {
        Self {
            phase,
            first_present: None,
            last_present: None,
            times: FrameTimes::new(),
            draws_without_texture: 0,
            saw_cpu_texture: false,
            textures: TextureStats::default(),
        }
    }

    /// Records a frame presented at `at`.
    pub fn presented(&mut self, at: Instant, frame: PresentedFrame) {
        if let Some(last) = self.last_present {
            self.times.record(at.saturating_duration_since(last));
        }
        self.first_present.get_or_insert(at);
        self.last_present = Some(at);
        if frame.pages_without_texture > 0 {
            self.draws_without_texture += 1;
        }
        self.saw_cpu_texture |= frame.cpu_textures > 0;
        let textures = &mut self.textures;
        textures.frames_received += u64::from(frame.frames_received);
        textures.popup_frames += u64::from(frame.popup_frames);
        textures.frames_dropped_for_pool_pressure +=
            u64::from(frame.frames_dropped_for_pool_pressure);
        textures.outstanding_textures = u64::from(frame.outstanding_textures);
        textures.max_outstanding_textures = textures
            .max_outstanding_textures
            .max(u64::from(frame.max_outstanding_textures));
    }

    /// Notes that the shell had nothing new to draw and presented nothing.
    /// The time until the next frame is then a rest, not a late frame, and
    /// is left out of the intervals.
    pub fn rested(&mut self) {
        self.last_present = None;
    }

    /// True once any frame in the phase used a CPU upload, which makes the
    /// run non-representative.
    pub fn saw_cpu_texture(&self) -> bool {
        self.saw_cpu_texture
    }

    /// Ends the phase; `budget` is the refresh interval.
    pub fn finish(self, budget: Duration) -> PhaseReport {
        let duration = match (self.first_present, self.last_present) {
            (Some(first), Some(last)) => last.saturating_duration_since(first),
            _ => Duration::ZERO,
        };
        PhaseReport {
            phase: self.phase,
            duration_ms: duration.as_secs_f64() * 1_000.0,
            frames: self.times.summary(budget),
            frames_received: Some(self.textures.frames_received),
            draws_without_texture: Some(self.draws_without_texture),
            textures: Some(self.textures),
            max_shapes_drawn: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUDGET: Duration = Duration::from_millis(8);

    fn recorded(intervals_ms: &[u64], missing_every: usize) -> PhaseRecorder {
        let mut recorder = PhaseRecorder::new(ProfileId::SlowPan);
        let mut at = Instant::now();
        recorder.presented(at, PresentedFrame::default());
        for (i, &ms) in intervals_ms.iter().enumerate() {
            at += Duration::from_millis(ms);
            let pages_without_texture = u32::from(missing_every > 0 && i % missing_every == 0);
            recorder.presented(
                at,
                PresentedFrame {
                    pages_without_texture,
                    frames_received: 2,
                    max_outstanding_textures: u32::try_from(i).unwrap_or(0),
                    ..PresentedFrame::default()
                },
            );
        }
        recorder
    }

    #[test]
    fn first_present_contributes_no_interval() {
        let report = recorded(&[8, 8, 8], 0).finish(BUDGET);
        assert_eq!(report.frames.frames, 3);
    }

    #[test]
    fn duration_spans_first_to_last_present() {
        let report = recorded(&[8, 9, 10], 0).finish(BUDGET);
        assert!((report.duration_ms - 27.0).abs() < 1e-6);
    }

    #[test]
    fn draws_without_texture_counts_frames_not_pages() {
        let report = recorded(&[8, 8, 8, 8], 2).finish(BUDGET);
        assert_eq!(report.draws_without_texture, Some(2));
    }

    #[test]
    fn a_turn_with_nothing_to_draw_is_not_a_long_frame() {
        let mut recorder = PhaseRecorder::new(ProfileId::ZoomOutThenPan);
        let start = Instant::now();
        recorder.presented(start, PresentedFrame::default());
        recorder.presented(start + BUDGET, PresentedFrame::default());
        // The camera stops changing for a while: nothing is presented.
        recorder.rested();
        let resumed = start + Duration::from_millis(300);
        recorder.presented(resumed, PresentedFrame::default());
        recorder.presented(resumed + BUDGET, PresentedFrame::default());
        let report = recorder.finish(BUDGET);
        assert_eq!((report.frames.frames, report.frames.long_frames), (2, 0));
    }
}
