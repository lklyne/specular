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
    }

    /// True once any frame in the phase used a CPU upload, which makes the
    /// run non-representative.
    pub fn saw_cpu_texture(&self) -> bool {
        self.saw_cpu_texture
    }

    /// Ends the phase. `budget` is the refresh interval; `textures` is the
    /// phase's share of the texture counters, when the runner tracks them.
    pub fn finish(self, budget: Duration, textures: Option<TextureStats>) -> PhaseReport {
        let duration = match (self.first_present, self.last_present) {
            (Some(first), Some(last)) => last.saturating_duration_since(first),
            _ => Duration::ZERO,
        };
        PhaseReport {
            phase: self.phase,
            duration_ms: duration.as_secs_f64() * 1_000.0,
            frames: self.times.summary(budget),
            frames_received: textures.map(|t| t.frames_received),
            draws_without_texture: Some(self.draws_without_texture),
            textures,
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
                    cpu_textures: 0,
                },
            );
        }
        recorder
    }

    #[test]
    fn first_present_contributes_no_interval() {
        let report = recorded(&[8, 8, 8], 0).finish(BUDGET, None);
        assert_eq!(report.frames.frames, 3);
    }

    #[test]
    fn duration_spans_first_to_last_present() {
        let report = recorded(&[8, 9, 10], 0).finish(BUDGET, None);
        assert!((report.duration_ms - 27.0).abs() < 1e-6);
    }

    #[test]
    fn draws_without_texture_counts_frames_not_pages() {
        let report = recorded(&[8, 8, 8, 8], 2).finish(BUDGET, None);
        assert_eq!(report.draws_without_texture, Some(2));
    }

    #[test]
    fn cpu_texture_is_remembered() {
        let mut recorder = PhaseRecorder::new(ProfileId::SlowZoom);
        recorder.presented(
            Instant::now(),
            PresentedFrame {
                pages_without_texture: 0,
                cpu_textures: 1,
            },
        );
        assert!(recorder.saw_cpu_texture());
    }

    #[test]
    fn empty_phase_finishes_with_zero_duration() {
        let report = PhaseRecorder::new(ProfileId::SlowPan).finish(BUDGET, None);
        assert!(report.duration_ms.abs() < f64::EPSILON && report.frames.frames == 0);
    }
}
