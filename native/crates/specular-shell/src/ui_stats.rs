//! What the two renderers in the window did, counted: how often each part
//! of the Kit's UI was built, how long each GPUI frame held the main
//! thread, how far apart the canvas's frames were, and how often the models
//! were read.
//!
//! The part counts are always kept, since a count is one add and a test
//! pins on it. The times are kept only between [`start`] and [`finish`],
//! which the UI benchmark calls around a profile.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

/// A part of the Kit's UI that is built on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Part {
    /// The root view: the slot and where the other parts go.
    Root,
    Toolbar,
    /// The popup hung from the toolbar.
    Popup,
    Sidebar,
    /// The right panel's header, or its list of threads.
    ChatHeader,
    Transcript,
    RunBar,
    Composer,
}

impl Part {
    /// Every part, in the order a report lists them.
    pub(crate) const ALL: [Self; 8] = [
        Self::Root,
        Self::Toolbar,
        Self::Popup,
        Self::Sidebar,
        Self::ChatHeader,
        Self::Transcript,
        Self::RunBar,
        Self::Composer,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Toolbar => "toolbar",
            Self::Popup => "popup",
            Self::Sidebar => "sidebar",
            Self::ChatHeader => "chatHeader",
            Self::Transcript => "transcript",
            Self::RunBar => "runBar",
            Self::Composer => "composer",
        }
    }
}

#[derive(Debug, Default)]
struct Stats {
    built: [u64; Part::ALL.len()],
    recording: Option<Recording>,
}

/// The times kept while a profile is measured.
#[derive(Debug)]
struct Recording {
    started: Instant,
    built_at_start: [u64; Part::ALL.len()],
    /// When the GPUI frame being drawn began.
    gpui_began: Option<Instant>,
    /// Each GPUI frame, in milliseconds from the root's render to the end
    /// of the update that drew and presented it.
    gpui_ms: Vec<f64>,
    canvas: Vec<Instant>,
    /// Each building of the models, in milliseconds.
    models_ms: Vec<f64>,
}

thread_local! {
    static STATS: RefCell<Stats> = RefCell::new(Stats::default());
}

fn with<R>(with: impl FnOnce(&mut Stats) -> R) -> R {
    STATS.with(|stats| with(&mut stats.borrow_mut()))
}

/// `part` was built: its elements were made again from its model.
pub(crate) fn built(part: Part) {
    with(|stats| stats.built[part as usize] += 1);
}

/// A GPUI frame began: the root view is being rendered. Returns whether
/// times are being kept, and so whether to say when the frame ends.
pub(crate) fn gpui_frame_began() -> bool {
    with(|stats| {
        let recording = stats.recording.as_mut();
        recording.is_some_and(|recording| {
            recording.gpui_began = Some(Instant::now());
            true
        })
    })
}

/// The update that drew the GPUI frame is over: it is laid out, painted
/// and handed to the display.
pub(crate) fn gpui_frame_ended() {
    with(|stats| {
        let Some(recording) = stats.recording.as_mut() else {
            return;
        };
        if let Some(began) = recording.gpui_began.take() {
            recording.gpui_ms.push(millis(began.elapsed()));
        }
    });
}

/// The canvas presented a frame.
pub(crate) fn canvas_presented() {
    with(|stats| {
        if let Some(recording) = stats.recording.as_mut() {
            recording.canvas.push(Instant::now());
        }
    });
}

/// The models were built from the app, which took `time`.
pub(crate) fn models_built(time: Duration) {
    with(|stats| {
        if let Some(recording) = stats.recording.as_mut() {
            recording.models_ms.push(millis(time));
        }
    });
}

fn millis(time: Duration) -> f64 {
    time.as_secs_f64() * 1_000.0
}

/// Starts keeping times.
pub(crate) fn start() {
    with(|stats| {
        stats.recording = Some(Recording {
            started: Instant::now(),
            built_at_start: stats.built,
            gpui_began: None,
            gpui_ms: Vec::new(),
            canvas: Vec::new(),
            models_ms: Vec::new(),
        });
    });
}

/// Stops keeping times and says what happened since [`start`], counting a
/// canvas frame long against a display refreshing every `refresh`.
pub(crate) fn finish(refresh: Duration) -> Option<Report> {
    with(|stats| {
        let recording = stats.recording.take()?;
        let seconds = recording.started.elapsed().as_secs_f64().max(1e-6);
        let mut built = [0.0; Part::ALL.len()];
        for (index, per_second) in built.iter_mut().enumerate() {
            let count = stats.built[index] - recording.built_at_start[index];
            *per_second = count as f64 / seconds;
        }
        let mut gaps: Vec<f64> = (recording.canvas.windows(2))
            .map(|pair| millis(pair[1].duration_since(pair[0])))
            .collect();
        let long = millis(refresh) * 1.5;
        let long_frames = gaps.iter().filter(|&&gap| gap > long).count();
        gaps.sort_by(f64::total_cmp);
        Some(Report {
            seconds,
            canvas_frames: recording.canvas.len(),
            canvas_p99_ms: quantile(&gaps, 0.99),
            canvas_max_ms: quantile(&gaps, 1.0),
            long_frames,
            gpui: Spread::of(recording.gpui_ms),
            models: Spread::of(recording.models_ms),
            built,
        })
    })
}

/// `sorted`'s value at `q` of the way through, or zero when it is empty.
fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

/// How many times something happened and how long it took, in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Spread {
    pub(crate) count: usize,
    pub(crate) mean: f64,
    pub(crate) p95: f64,
    pub(crate) max: f64,
}

impl Spread {
    fn of(mut times: Vec<f64>) -> Self {
        if times.is_empty() {
            return Self::default();
        }
        times.sort_by(f64::total_cmp);
        Self {
            count: times.len(),
            mean: times.iter().sum::<f64>() / times.len() as f64,
            p95: quantile(&times, 0.95),
            max: quantile(&times, 1.0),
        }
    }
}

/// One measured profile.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Report {
    pub(crate) seconds: f64,
    pub(crate) canvas_frames: usize,
    pub(crate) canvas_p99_ms: f64,
    pub(crate) canvas_max_ms: f64,
    /// Canvas intervals over one and a half refreshes.
    pub(crate) long_frames: usize,
    pub(crate) gpui: Spread,
    pub(crate) models: Spread,
    /// How many times a second each part was built, in [`Part::ALL`]'s order.
    pub(crate) built: [f64; Part::ALL.len()],
}

impl Report {
    /// The report's numbers as the fields of a JSON object, without the
    /// braces, so the caller can say what was measured beside them.
    pub(crate) fn json_fields(&self) -> String {
        let per_second = |count: usize| count as f64 / self.seconds;
        let mut out = String::new();
        let _ = write!(
            out,
            "\"seconds\":{:.2},\"canvasFps\":{:.1},\"canvasP99Ms\":{:.2},\"canvasMaxMs\":{:.2},\
             \"longFrames\":{},\"gpuiFps\":{:.1},\"gpuiFrameMs\":{:.3},\"gpuiFrameP95Ms\":{:.3},\
             \"gpuiFrameMaxMs\":{:.3},\"modelReadsPerSecond\":{:.1},\"modelReadMs\":{:.3},\
             \"builtPerSecond\":{{",
            self.seconds,
            per_second(self.canvas_frames),
            self.canvas_p99_ms,
            self.canvas_max_ms,
            self.long_frames,
            per_second(self.gpui.count),
            self.gpui.mean,
            self.gpui.p95,
            self.gpui.max,
            per_second(self.models.count),
            self.models.mean,
        );
        for (index, part) in Part::ALL.iter().enumerate() {
            let comma = if index == 0 { "" } else { "," };
            let _ = write!(out, "{comma}\"{}\":{:.1}", part.name(), self.built[index]);
        }
        out.push('}');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_counts_what_happened_between_start_and_finish_and_nothing_before() {
        built(Part::Toolbar);
        canvas_presented();
        start();
        for _ in 0..3 {
            built(Part::RunBar);
            gpui_frame_began();
            gpui_frame_ended();
            canvas_presented();
        }
        // An end with no beginning is some other update, not a frame.
        gpui_frame_ended();
        models_built(Duration::from_millis(2));
        let report = finish(Duration::from_millis(8)).unwrap();
        assert_eq!((report.canvas_frames, report.gpui.count), (3, 3));
        assert_eq!(report.models.count, 1);
        assert!((report.models.mean - 2.0).abs() < 1e-9);
        assert!(report.built[Part::Toolbar as usize].abs() < f64::EPSILON);
        assert!(report.built[Part::RunBar as usize] > 0.0);
        assert!(report.json_fields().contains("\"runBar\":"));
        assert_eq!(finish(Duration::from_millis(8)), None);
    }
}
