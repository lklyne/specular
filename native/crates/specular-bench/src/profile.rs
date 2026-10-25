//! Gesture profiles mirroring `src/shared/pan-zoom-perf-test.ts`.

use std::{fmt, str::FromStr, time::Duration};

use glam::Vec2;
use serde::{Deserialize, Serialize};
use specular_core::ViewportInputDelta;

use crate::BenchError;

/// Default interval between steps (`PAN_ZOOM_PERF_FRAME_MS`); real runs use
/// the display refresh interval instead, as the Electron test does.
pub const STEP_INTERVAL: Duration = Duration::from_millis(16);
/// Idle gap between profiles (`PAN_ZOOM_PERF_PHASE_GAP_MS`).
pub const PHASE_GAP: Duration = Duration::from_millis(250);

/// Fraction of a step below which a duration counts as a whole number of
/// steps; far above nanosecond truncation error, far below a real remainder.
const STEP_ROUNDING_TOLERANCE: f64 = 1e-3;

/// Share of `zoom-out-then-pan` spent zooming before the pan starts.
const ZOOM_LEAD_FRACTION: f64 = 1.0 / 6.0;

/// Stable profile identifiers, serialized with the Electron ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProfileId {
    /// `slow-pan`.
    SlowPan,
    /// `slow-zoom`.
    SlowZoom,
    /// `fast-diagonal-pan`.
    FastDiagonalPan,
    /// `slow-pan-zoom`.
    SlowPanZoom,
    /// `fast-pan-zoom`.
    FastPanZoom,
    /// `zoom-out-then-pan`: a quick zoom-out, then a fast pan still running
    /// when a zoom settle would land.
    ZoomOutThenPan,
    /// `idle`: nothing moves. Not one of Electron's, and not part of `all`.
    Idle,
}

impl ProfileId {
    /// Electron's ids, in run order.
    pub const ALL: [Self; 6] = [
        Self::SlowPan,
        Self::SlowZoom,
        Self::FastDiagonalPan,
        Self::SlowPanZoom,
        Self::FastPanZoom,
        Self::ZoomOutThenPan,
    ];

    /// The Electron id (`slow-pan`, ...).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SlowPan => "slow-pan",
            Self::SlowZoom => "slow-zoom",
            Self::FastDiagonalPan => "fast-diagonal-pan",
            Self::SlowPanZoom => "slow-pan-zoom",
            Self::FastPanZoom => "fast-pan-zoom",
            Self::ZoomOutThenPan => "zoom-out-then-pan",
            Self::Idle => "idle",
        }
    }
}

impl fmt::Display for ProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ProfileId {
    type Err = BenchError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "idle" => Ok(Self::Idle),
            _ => Self::ALL
                .into_iter()
                .find(|id| id.as_str() == s)
                .ok_or_else(|| BenchError::UnknownProfile(s.to_owned())),
        }
    }
}

/// One scripted gesture: total pan and zoom spread evenly over `duration`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct GestureProfile {
    /// Identifier.
    pub id: ProfileId,
    /// Human label.
    pub label: &'static str,
    /// Wall-clock length.
    pub duration: Duration,
    /// Total screen-space pan in logical pixels.
    pub pan: Vec2,
    /// Total wheel-style zoom delta (negative zooms in).
    pub zoom_delta_y: f32,
}

/// The six profiles, in run order, with the Electron values.
pub const PROFILES: [GestureProfile; 6] = [
    GestureProfile {
        id: ProfileId::SlowPan,
        label: "Slow pan",
        duration: Duration::from_millis(2_000),
        pan: Vec2::new(360.0, 0.0),
        zoom_delta_y: 0.0,
    },
    GestureProfile {
        id: ProfileId::SlowZoom,
        label: "Slow zoom",
        duration: Duration::from_millis(2_000),
        pan: Vec2::ZERO,
        zoom_delta_y: -140.0,
    },
    GestureProfile {
        id: ProfileId::FastDiagonalPan,
        label: "Fast diagonal pan",
        duration: Duration::from_millis(450),
        pan: Vec2::new(-360.0, -280.0),
        zoom_delta_y: 0.0,
    },
    GestureProfile {
        id: ProfileId::SlowPanZoom,
        label: "Slow pan + zoom",
        duration: Duration::from_millis(2_000),
        pan: Vec2::new(300.0, 180.0),
        zoom_delta_y: 100.0,
    },
    GestureProfile {
        id: ProfileId::FastPanZoom,
        label: "Fast pan + zoom",
        duration: Duration::from_millis(450),
        pan: Vec2::new(-340.0, 220.0),
        zoom_delta_y: -120.0,
    },
    GestureProfile {
        id: ProfileId::ZoomOutThenPan,
        label: "Zoom out, then fast pan",
        duration: Duration::from_millis(2_400),
        pan: Vec2::new(-900.0, 260.0),
        zoom_delta_y: 320.0,
    },
];

/// A camera left alone. What a frame costs when nothing changes, which
/// should be nothing: no frame drawn. Runs only when named.
pub const IDLE: GestureProfile = GestureProfile {
    id: ProfileId::Idle,
    label: "Idle",
    duration: Duration::from_millis(5_000),
    pan: Vec2::ZERO,
    zoom_delta_y: 0.0,
};

impl GestureProfile {
    /// The profile with its duration replaced, as `durationMs` does on
    /// `POST /perf/pan-zoom/run`. The totals are unchanged, so a shorter
    /// duration is a faster gesture.
    #[must_use]
    pub fn with_duration(self, duration: Duration) -> Self {
        Self { duration, ..self }
    }

    /// Number of steps [`build_steps`] yields at `interval`.
    pub fn step_count(&self, interval: Duration) -> usize {
        let interval_ms = interval.as_secs_f64() * 1_000.0;
        let duration_ms = self.duration.as_secs_f64() * 1_000.0;
        if interval_ms > 0.0 {
            // `Duration` holds whole nanoseconds, so a 120 Hz interval is
            // 8_333_333 ns and 2000 ms spans 240.00001 of them; a plain ceil
            // would add a step the TypeScript (float ms) never takes.
            let steps = duration_ms / interval_ms - STEP_ROUNDING_TOLERANCE;
            (steps.ceil() as usize).max(1)
        } else {
            1
        }
    }

    /// Wall-clock time the profile occupies when each step is followed by a
    /// wait of `interval` (the Electron loop applies, then waits).
    pub fn planned_duration(&self, interval: Duration) -> Duration {
        interval.saturating_mul(u32::try_from(self.step_count(interval)).unwrap_or(u32::MAX))
    }
}

/// The profiles a run executes, mirroring `runPanZoomPerfTest` options: an
/// empty `ids` selects Electron's six; selection keeps [`PROFILES`] order
/// regardless of the order of `ids`, with [`IDLE`] last when it is named; a
/// non-zero `duration` overrides every profile's.
pub fn select_profiles(ids: &[ProfileId], duration: Option<Duration>) -> Vec<GestureProfile> {
    PROFILES
        .into_iter()
        .filter(|profile| ids.is_empty() || ids.contains(&profile.id))
        .chain(ids.contains(&ProfileId::Idle).then_some(IDLE))
        .map(|profile| match duration {
            Some(duration) if !duration.is_zero() => profile.with_duration(duration),
            _ => profile,
        })
        .collect()
}

/// One tick of a profile: the delta to apply, then wait one step interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GestureStep {
    /// Screen-space pan for this tick.
    pub pan: Vec2,
    /// Wheel-style zoom delta for this tick.
    pub zoom_delta_y: f32,
}

impl GestureStep {
    /// The camera input for this step, zooming about `anchor` (the Electron
    /// test anchors at the canvas centre).
    pub fn to_input(self, anchor: Option<Vec2>) -> ViewportInputDelta {
        ViewportInputDelta {
            pan: self.pan,
            zoom_delta_y: self.zoom_delta_y,
            anchor,
        }
    }
}

/// Expands a profile into per-tick steps at `interval`
/// (`buildPanZoomPerfSteps`).
pub fn build_steps(profile: &GestureProfile, interval: Duration) -> Vec<GestureStep> {
    let step_count = profile.step_count(interval);
    if profile.id == ProfileId::ZoomOutThenPan {
        let zoom_steps = ((step_count as f64 * ZOOM_LEAD_FRACTION).round() as usize).max(1);
        let pan_steps = step_count.saturating_sub(zoom_steps).max(1);
        let zoom = GestureStep {
            pan: Vec2::ZERO,
            zoom_delta_y: profile.zoom_delta_y / zoom_steps as f32,
        };
        let pan = GestureStep {
            pan: profile.pan / pan_steps as f32,
            zoom_delta_y: 0.0,
        };
        return std::iter::repeat_n(zoom, zoom_steps)
            .chain(std::iter::repeat_n(pan, pan_steps))
            .collect();
    }
    let step = GestureStep {
        pan: profile.pan / step_count as f32,
        zoom_delta_y: profile.zoom_delta_y / step_count as f32,
    };
    vec![step; step_count]
}
