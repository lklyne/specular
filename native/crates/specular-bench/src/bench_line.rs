//! The JSON lines `specular-app` prints on stdout, which `assemble` folds
//! into a [`RunReport`](crate::RunReport). One type on both sides, so a field
//! renamed in the app fails `assemble` instead of reading as zero.

use serde::{Deserialize, Serialize};

use crate::{LatencySummary, PaintPolicy, PhaseReport};

/// One line of `specular-app` output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[expect(
    clippy::large_enum_variant,
    reason = "one value per printed line, never stored in bulk"
)]
pub enum BenchLine {
    /// One gesture profile's result (`--bench`).
    Profile(ProfileLine),
    /// The session's forwarded-input latency, printed at exit when any input
    /// reached a page.
    InputLatency(InputLatencyLine),
}

/// One profile's result: the [`PhaseReport`] fields at the top level, plus
/// what only the Rust shell knows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileLine {
    /// Frame timing and texture counters.
    #[serde(flatten)]
    pub phase: PhaseReport,
    /// The profile's human-readable label.
    pub label: String,
    /// Page source name (`cef`, `synthetic`).
    pub source: String,
    /// Pages on the canvas.
    pub pages: usize,
    /// False when the source is synthetic or any frame drawn came from a CPU
    /// upload: such runs must not be compared against Electron (ADR 0038).
    pub representative: bool,
    /// Refresh interval the profile was stepped at.
    pub step_interval_ms: f64,
    /// Longest wait between a paint arriving and the frame showing it.
    pub max_paint_to_submit_ms: Option<f64>,
    /// The page paint policy the shell applied (frame-rate tiers, culling),
    /// so `compare` can flag runs whose shells did different work.
    pub paint_policy: PaintPolicy,
}

/// Forwarded input -> page repaint -> presented, over a whole session.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputLatencyLine {
    /// The latency distribution.
    pub input_latency: LatencySummary,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FrameSummary, ProfileId};

    fn profile_line() -> ProfileLine {
        ProfileLine {
            phase: PhaseReport {
                phase: ProfileId::SlowZoom,
                duration_ms: 2_000.0,
                frames: FrameSummary {
                    frames: 240,
                    ..FrameSummary::default()
                },
                frames_received: None,
                draws_without_texture: Some(0),
                textures: None,
            },
            label: "Slow zoom".to_owned(),
            source: "cef".to_owned(),
            pages: 9,
            representative: true,
            step_interval_ms: 8.33,
            max_paint_to_submit_ms: None,
            paint_policy: PaintPolicy::ElectronLod,
        }
    }

    #[test]
    fn profile_line_round_trips() {
        let line = BenchLine::Profile(profile_line());
        let json = serde_json::to_string(&line).unwrap();
        assert_eq!(serde_json::from_str::<BenchLine>(&json).unwrap(), line);
    }

    #[test]
    fn profile_line_puts_phase_fields_at_top_level() {
        let json = serde_json::to_value(BenchLine::Profile(profile_line())).unwrap();
        assert_eq!(json["draws"], 240);
    }

    #[test]
    fn input_latency_line_round_trips() {
        let line = BenchLine::InputLatency(InputLatencyLine {
            input_latency: LatencySummary {
                samples: 3,
                ..LatencySummary::default()
            },
        });
        let json = serde_json::to_string(&line).unwrap();
        assert_eq!(serde_json::from_str::<BenchLine>(&json).unwrap(), line);
    }

    #[test]
    fn profile_line_missing_a_field_is_rejected() {
        let mut json = serde_json::to_value(BenchLine::Profile(profile_line())).unwrap();
        if let Some(object) = json.as_object_mut() {
            object.remove("stepIntervalMs");
        }
        assert!(serde_json::from_value::<BenchLine>(json).is_err());
    }
}
