//! Gesture profiles mirroring `src/shared/pan-zoom-perf-test.ts`.

use std::time::Duration;

use glam::Vec2;
use serde::Serialize;
use specular_core::ViewportInputDelta;

/// Default interval between steps (`PAN_ZOOM_PERF_FRAME_MS`); real runs use
/// the display refresh interval instead, as the Electron test does.
pub const STEP_INTERVAL: Duration = Duration::from_millis(16);
/// Idle gap between profiles (`PAN_ZOOM_PERF_PHASE_GAP_MS`).
pub const PHASE_GAP: Duration = Duration::from_millis(250);

/// Share of `zoom-out-then-pan` spent zooming before the pan starts.
const ZOOM_LEAD_FRACTION: f64 = 1.0 / 6.0;

/// Stable profile identifiers, serialized with the Electron ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
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
#[expect(
    clippy::duration_suboptimal_units,
    reason = "milliseconds match the TypeScript source value for value"
)]
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
    let interval_ms = interval.as_secs_f64() * 1_000.0;
    let duration_ms = profile.duration.as_secs_f64() * 1_000.0;
    let step_count = if interval_ms > 0.0 {
        ((duration_ms / interval_ms).ceil() as usize).max(1)
    } else {
        1
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: ProfileId) -> GestureProfile {
        PROFILES.into_iter().find(|p| p.id == id).unwrap()
    }

    #[test]
    fn slow_pan_at_16ms_has_125_steps() {
        assert_eq!(
            build_steps(&profile(ProfileId::SlowPan), STEP_INTERVAL).len(),
            125
        );
    }

    #[test]
    fn steps_sum_to_profile_totals() {
        let p = profile(ProfileId::SlowPanZoom);
        let steps = build_steps(&p, STEP_INTERVAL);
        let pan: Vec2 = steps.iter().map(|s| s.pan).sum();
        assert!(pan.abs_diff_eq(p.pan, 1e-2));
    }

    #[test]
    fn zoom_out_then_pan_zooms_before_panning() {
        let steps = build_steps(&profile(ProfileId::ZoomOutThenPan), STEP_INTERVAL);
        // 150 steps: round(150 / 6) = 25 zoom steps lead.
        assert!(steps[24].zoom_delta_y > 0.0 && steps[25].zoom_delta_y == 0.0);
    }

    #[test]
    fn profile_ids_serialize_with_electron_names() {
        let json = serde_json::to_string(&ProfileId::FastDiagonalPan).unwrap();
        assert_eq!(json, "\"fast-diagonal-pan\"");
    }
}
