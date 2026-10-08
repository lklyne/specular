//! Reduces a Chromium trace recorded by Electron's `POST /perf/pan-zoom/run`
//! to the same per-profile frame summary the Rust runner produces.
//!
//! Presented frames are the `Display::DrawAndSwap` slices on the viz
//! compositor thread: one per frame the browser composites for the window.
//! The trace carries no phase markers, so phases are recovered from the idle
//! gaps the test leaves between them (`PAN_ZOOM_PERF_PHASE_GAP_MS` = 250 ms):
//! presents are split into bursts wherever consecutive presents are further
//! apart than `gap`, and the run of bursts whose lengths best match the
//! profiles' planned durations is assigned to them in order. That only works
//! when nothing presents during the gaps — static pages, nothing animating.
//! For animated fixtures, run one profile per request (`"profiles":
//! ["slow-pan"]`) and select that profile here.
//!
//! Unverified assumption: under ADR 0038 every offscreen page window has its
//! own `ui::Compositor` and `viz::Display`, drawing on the same viz thread as
//! the canvas window. If those draws are `DrawAndSwap` slices too, a burst
//! holds far more presents than the profile has steps, fps reads high and
//! frame times low. [`present_count_warning`] flags that per phase; until a
//! real trace shows about steps + 1 presents per burst, Electron numbers
//! from this module are unverified.

use std::{collections::HashMap, time::Duration};

use serde::{Deserialize, Deserializer};

use crate::{BenchError, FrameTimes, GestureProfile, PhaseReport};

/// The slice marking one composited frame.
pub const PRESENT_EVENT: &str = "Display::DrawAndSwap";
/// Thread the presents are read from unless told otherwise.
pub const DEFAULT_THREAD: &str = "VizCompositorThread";
/// Presents per expected present above which a burst cannot be the canvas
/// window's alone (one present per step, plus the first).
const MAX_PRESENTS_PER_EXPECTED: f64 = 1.25;
/// Bursts with fewer presents than this are paints, not gestures (a page
/// settling, the camera restore after the last profile).
const MIN_BURST_PRESENTS: usize = 5;

#[derive(Debug, Deserialize)]
struct RawEvent {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    ph: Option<String>,
    #[serde(default)]
    ts: Option<f64>,
    #[serde(default)]
    pid: Option<i64>,
    #[serde(default)]
    tid: Option<i64>,
    #[serde(default)]
    args: Option<RawArgs>,
}

#[derive(Debug, Deserialize)]
struct RawArgs {
    #[serde(default, deserialize_with = "string_or_none")]
    name: Option<String>,
}

/// `args.name` is a string on `thread_name` metadata but can be anything on
/// other events; only strings matter here.
fn string_or_none<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(s) => Some(s),
        _ => None,
    })
}

#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(rename = "traceEvents")]
    trace_events: Vec<RawEvent>,
}

/// Presents found on one thread, in microseconds of trace time.
#[derive(Debug, Clone, PartialEq)]
pub struct PresentTimeline {
    /// Name of the thread read.
    pub thread: String,
    /// Sorted `ts` of every present.
    pub presents_us: Vec<f64>,
}

/// Parses a Chrome-JSON trace (the `{ "traceEvents": [...] }` envelope or a
/// bare array) and returns the presents on the busiest thread named
/// `thread_name`.
pub fn read_presents(trace: &str, thread_name: &str) -> Result<PresentTimeline, BenchError> {
    let json_error = |source| BenchError::Json {
        what: "trace".to_owned(),
        source,
    };
    let events: Vec<RawEvent> = if trace.trim_start().starts_with('[') {
        serde_json::from_str(trace).map_err(json_error)?
    } else {
        serde_json::from_str::<Envelope>(trace)
            .map_err(json_error)?
            .trace_events
    };

    let mut thread_names: HashMap<(i64, i64), String> = HashMap::new();
    let mut presents: HashMap<(i64, i64), Vec<f64>> = HashMap::new();
    for event in events {
        let (Some(pid), Some(tid)) = (event.pid, event.tid) else {
            continue;
        };
        let phase = event.ph.as_deref();
        match event.name.as_deref() {
            Some("thread_name") if phase == Some("M") => {
                if let Some(name) = event.args.and_then(|args| args.name) {
                    thread_names.insert((pid, tid), name);
                }
            }
            Some(PRESENT_EVENT) if matches!(phase, Some("X" | "B")) => {
                if let Some(ts) = event.ts {
                    presents.entry((pid, tid)).or_default().push(ts);
                }
            }
            _ => {}
        }
    }

    let (key, mut presents_us) = presents
        .into_iter()
        .filter(|(key, _)| {
            thread_names
                .get(key)
                .is_some_and(|name| name == thread_name)
        })
        .max_by_key(|(_, list)| list.len())
        .ok_or(BenchError::NoPresents {
            event: PRESENT_EVENT,
        })?;
    presents_us.sort_by(f64::total_cmp);
    Ok(PresentTimeline {
        thread: thread_names.remove(&key).unwrap_or_default(),
        presents_us,
    })
}

/// Splits sorted present times into bursts separated by more than `gap`,
/// dropping bursts too short to be a gesture.
pub fn split_bursts(presents_us: &[f64], gap: Duration) -> Vec<&[f64]> {
    let gap_us = gap.as_secs_f64() * 1e6;
    presents_us
        .chunk_by(|a, b| b - a <= gap_us)
        .filter(|burst| burst.len() >= MIN_BURST_PRESENTS)
        .collect()
}

/// Picks the contiguous run of `profiles.len()` bursts whose lengths best
/// match the profiles' planned durations at `interval`.
pub fn assign_bursts<'a>(
    bursts: &[&'a [f64]],
    profiles: &[GestureProfile],
    interval: Duration,
) -> Result<Vec<&'a [f64]>, BenchError> {
    let wanted = profiles.len();
    if bursts.len() < wanted || wanted == 0 {
        return Err(BenchError::Segmentation {
            found: bursts.len(),
            expected: wanted,
        });
    }
    let error_of = |window: &[&[f64]]| -> f64 {
        window
            .iter()
            .zip(profiles)
            .map(|(burst, profile)| {
                let planned = profile.planned_duration(interval).as_secs_f64() * 1e6;
                (burst_length_us(burst) - planned).abs()
            })
            .sum()
    };
    bursts
        .windows(wanted)
        .min_by(|a, b| error_of(a).total_cmp(&error_of(b)))
        .map(<[&[f64]]>::to_vec)
        .ok_or(BenchError::Segmentation {
            found: bursts.len(),
            expected: wanted,
        })
}

fn burst_length_us(burst: &[f64]) -> f64 {
    match (burst.first(), burst.last()) {
        (Some(first), Some(last)) => last - first,
        _ => 0.0,
    }
}

/// Summarises one burst as `profile`'s phase against the refresh `budget`.
pub fn phase_from_burst(profile: &GestureProfile, burst: &[f64], budget: Duration) -> PhaseReport {
    let mut times = FrameTimes::new();
    for pair in burst.windows(2) {
        times.record(Duration::from_secs_f64((pair[1] - pair[0]).max(0.0) / 1e6));
    }
    PhaseReport {
        phase: profile.id,
        duration_ms: burst_length_us(burst) / 1_000.0,
        frames: times.summary(budget),
        frames_received: None,
        draws_without_texture: None,
        textures: None,
        max_shapes_drawn: None,
    }
}

/// A warning when `phase` holds implausibly many presents for `profile`
/// stepped at `interval`: about `steps + 1` are expected from the canvas
/// window, and many more mean other displays' draws were counted.
pub fn present_count_warning(
    profile: &GestureProfile,
    phase: &PhaseReport,
    interval: Duration,
) -> Option<String> {
    let expected = profile.step_count(interval) + 1;
    let presents = phase.frames.frames + 1;
    (presents as f64 > expected as f64 * MAX_PRESENTS_PER_EXPECTED).then(|| {
        format!(
            "{}: {presents} presents for ~{expected} expected; `{PRESENT_EVENT}` is likely \
             counting offscreen page displays too, so these frame times are not the canvas \
             window's",
            profile.id
        )
    })
}

/// Reads presents from `trace` and reduces them to one phase per profile.
pub fn phases_from_trace(
    trace: &str,
    thread_name: &str,
    profiles: &[GestureProfile],
    budget: Duration,
    gap: Duration,
) -> Result<(PresentTimeline, Vec<PhaseReport>), BenchError> {
    let timeline = read_presents(trace, thread_name)?;
    let bursts = split_bursts(&timeline.presents_us, gap);
    let assigned = assign_bursts(&bursts, profiles, budget)?;
    let phases = assigned
        .iter()
        .zip(profiles)
        .map(|(burst, profile)| phase_from_burst(profile, burst, budget))
        .collect();
    Ok((timeline, phases))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProfileId, select_profiles};

    const BUDGET: Duration = Duration::from_millis(8);
    const GAP: Duration = Duration::from_millis(200);

    /// `count` presents every `step_us`, starting at `start_us`.
    fn burst(start_us: f64, count: usize, step_us: f64) -> Vec<f64> {
        (0..count).map(|i| start_us + i as f64 * step_us).collect()
    }

    fn trace_json(presents: &[f64], other_thread: &[f64]) -> String {
        let mut events = vec![
            r#"{"name":"thread_name","ph":"M","pid":1,"tid":7,"args":{"name":"VizCompositorThread"}}"#.to_owned(),
            r#"{"name":"thread_name","ph":"M","pid":1,"tid":8,"args":{"name":"CrBrowserMain"}}"#.to_owned(),
            r#"{"name":"Other","ph":"X","pid":1,"tid":7,"ts":5,"dur":1,"args":{"name":42}}"#.to_owned(),
        ];
        events.extend(presents.iter().map(|ts| {
            format!(
                r#"{{"name":"Display::DrawAndSwap","ph":"X","pid":1,"tid":7,"ts":{ts},"dur":100}}"#
            )
        }));
        events.extend(other_thread.iter().map(|ts| {
            format!(
                r#"{{"name":"Display::DrawAndSwap","ph":"X","pid":1,"tid":8,"ts":{ts},"dur":100}}"#
            )
        }));
        format!(r#"{{"traceEvents":[{}]}}"#, events.join(","))
    }

    #[test]
    fn read_presents_takes_only_the_named_thread() {
        let trace = trace_json(&burst(0.0, 10, 8_000.0), &burst(0.0, 50, 1_000.0));
        assert_eq!(
            read_presents(&trace, DEFAULT_THREAD)
                .unwrap()
                .presents_us
                .len(),
            10
        );
    }

    #[test]
    fn read_presents_accepts_bare_array() {
        let trace = trace_json(&burst(0.0, 3, 8_000.0), &[]);
        let bare = trace
            .trim_start_matches(r#"{"traceEvents":"#)
            .trim_end_matches('}');
        assert_eq!(
            read_presents(bare, DEFAULT_THREAD)
                .unwrap()
                .presents_us
                .len(),
            3
        );
    }

    #[test]
    fn split_bursts_cuts_at_gaps_and_drops_short_bursts() {
        let mut presents = burst(0.0, 20, 8_000.0);
        presents.extend(burst(500_000.0, 2, 8_000.0)); // a lone repaint
        presents.extend(burst(1_000_000.0, 30, 8_000.0));
        let lengths: Vec<usize> = split_bursts(&presents, GAP)
            .iter()
            .map(|b| b.len())
            .collect();
        assert_eq!(lengths, [20, 30]);
    }

    #[test]
    fn assign_bursts_skips_a_leading_warmup_burst() {
        let profiles = select_profiles(&[ProfileId::SlowPan, ProfileId::FastDiagonalPan], None);
        // Warmup paint (100 ms), slow pan (2000 ms), fast pan (450 ms).
        let warmup = burst(0.0, 13, 8_000.0);
        let slow = burst(1_000_000.0, 251, 8_000.0);
        let fast = burst(3_500_000.0, 57, 8_000.0);
        let bursts = [warmup.as_slice(), slow.as_slice(), fast.as_slice()];
        let assigned = assign_bursts(&bursts, &profiles, BUDGET).unwrap();
        assert_eq!(assigned[0].len(), 251);
    }

    #[test]
    fn phase_from_burst_measures_intervals_against_budget() {
        let profile = select_profiles(&[ProfileId::SlowPan], None)[0];
        let mut presents = burst(0.0, 10, 8_000.0);
        presents.push(presents[9] + 20_000.0); // one long frame
        let phase = phase_from_burst(&profile, &presents, BUDGET);
        assert_eq!((phase.frames.frames, phase.frames.long_frames), (10, 1));
    }

    #[test]
    fn presents_from_many_displays_raise_a_warning() {
        let profile = select_profiles(&[ProfileId::SlowPan], None)[0];
        let steps = profile.step_count(BUDGET);
        // Twenty page windows drawing alongside the canvas window.
        let presents = burst(0.0, (steps + 1) * 20, 400.0);
        let phase = phase_from_burst(&profile, &presents, BUDGET);
        assert!(present_count_warning(&profile, &phase, BUDGET).is_some());
    }

    #[test]
    fn phases_from_trace_assigns_each_profile_its_burst() {
        let profiles = select_profiles(&[ProfileId::SlowPan, ProfileId::FastDiagonalPan], None);
        let mut presents = burst(0.0, 251, 8_000.0);
        presents.extend(burst(2_400_000.0, 57, 8_000.0));
        let trace = trace_json(&presents, &[]);
        let (_, phases) =
            phases_from_trace(&trace, DEFAULT_THREAD, &profiles, BUDGET, GAP).unwrap();
        let ids: Vec<ProfileId> = phases.iter().map(|p| p.phase).collect();
        assert_eq!(ids, [ProfileId::SlowPan, ProfileId::FastDiagonalPan]);
    }
}
