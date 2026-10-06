//! Side-by-side markdown comparison of two results files.
//!
//! Loading is deliberately lenient so every producer compares without a
//! conversion step: a [`RunReport`](crate::RunReport), a bare array of phase
//! objects (the ADR 0038 lab's `phases`), or JSON lines of phase objects (the
//! app's `--bench` output). Phase metrics are looked up at the top level of a
//! phase object, then under `frames`, then under `textures`. A profile that
//! appears several times — three runs concatenated, as the spike plan asks —
//! is reduced to the median of each metric.

use std::{collections::BTreeMap, fmt::Write as _};

use serde_json::Value;

use crate::{BenchError, ProfileId, Shell};

/// One metric row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Metric {
    /// JSON key (phase metrics) or JSON pointer (run metrics).
    key: &'static str,
    label: &'static str,
}

const PHASE_METRICS: [Metric; 12] = [
    Metric {
        key: "drawFps",
        label: "drawFps",
    },
    Metric {
        key: "meanFrameMs",
        label: "mean ms",
    },
    Metric {
        key: "p50FrameMs",
        label: "p50 ms",
    },
    Metric {
        key: "p95FrameMs",
        label: "p95 ms",
    },
    Metric {
        key: "p99FrameMs",
        label: "p99 ms",
    },
    Metric {
        key: "maxFrameMs",
        label: "max ms",
    },
    Metric {
        key: "longFrames",
        label: "longFrames",
    },
    Metric {
        key: "drawsWithoutTexture",
        label: "draws without texture",
    },
    Metric {
        key: "framesWithoutTexture",
        label: "framesWithoutTexture",
    },
    Metric {
        key: "framesDroppedForPoolPressure",
        label: "framesDroppedForPoolPressure",
    },
    Metric {
        key: "maxOutstandingTextures",
        label: "maxOutstandingTextures",
    },
    Metric {
        key: "maxShapesDrawn",
        label: "maxShapesDrawn",
    },
];

const RUN_METRICS: [Metric; 12] = [
    Metric {
        key: "/memory/idle/footprintMb",
        label: "footprint idle (MB)",
    },
    Metric {
        key: "/memory/end/footprintMb",
        label: "footprint end (MB)",
    },
    Metric {
        key: "/memory/peak/footprintMb",
        label: "footprint peak (MB)",
    },
    Metric {
        key: "/memory/idle/rssMb",
        label: "RSS idle (MB)",
    },
    Metric {
        key: "/memory/end/rssMb",
        label: "RSS end (MB)",
    },
    Metric {
        key: "/memory/peak/rssMb",
        label: "RSS peak (MB)",
    },
    Metric {
        key: "/memory/end/processes",
        label: "processes",
    },
    Metric {
        key: "/inputLatency/p50Ms",
        label: "input latency p50 ms",
    },
    Metric {
        key: "/inputLatency/p95Ms",
        label: "input latency p95 ms",
    },
    Metric {
        key: "/textures/framesWithoutTexture",
        label: "framesWithoutTexture",
    },
    Metric {
        key: "/textures/framesDroppedForPoolPressure",
        label: "framesDroppedForPoolPressure",
    },
    Metric {
        key: "/textures/maxOutstandingTextures",
        label: "maxOutstandingTextures",
    },
];

/// A results file, normalised for comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedRun {
    label: String,
    root: Value,
    phases: BTreeMap<ProfileId, Vec<Value>>,
}

impl LoadedRun {
    /// Parses `text`; `name` (usually the file name) labels the column when
    /// the file does not name its shell.
    pub fn parse(text: &str, name: &str) -> Result<Self, BenchError> {
        let (root, phase_values) = match serde_json::from_str::<Value>(text) {
            Ok(Value::Array(items)) => (Value::Null, items),
            Ok(Value::Object(map)) => {
                let object = Value::Object(map);
                if let Some(Value::Array(items)) = object.get("phases") {
                    let items = items.clone();
                    (object, items)
                } else if object.get("tracePath").is_some() {
                    return Err(BenchError::NoPhases(format!(
                        "{name} (a /perf/pan-zoom/run response; convert its trace with \
                         `specular-bench electron-trace` first)"
                    )));
                } else {
                    (Value::Null, vec![object])
                }
            }
            Ok(_) => return Err(BenchError::NoPhases(name.to_owned())),
            Err(_) => {
                // The app's session-wide latency line carries no phase.
                let mut lines = parse_json_lines(text, name)?;
                lines.retain(|line| line.get("inputLatency").is_none());
                (Value::Null, lines)
            }
        };
        let mut phases: BTreeMap<ProfileId, Vec<Value>> = BTreeMap::new();
        for value in phase_values {
            let id = phase_id(&value).ok_or_else(|| BenchError::NoPhases(name.to_owned()))??;
            phases.entry(id).or_default().push(value);
        }
        if phases.is_empty() {
            return Err(BenchError::NoPhases(name.to_owned()));
        }
        let label = root
            .get("shell")
            .and_then(|shell| serde_json::from_value::<Shell>(shell.clone()).ok())
            .map_or_else(|| name.to_owned(), |shell| shell.label().to_owned());
        Ok(Self {
            label,
            root,
            phases,
        })
    }

    /// Column heading.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Median of `key` over every recorded instance of `phase`.
    fn phase_metric(&self, phase: ProfileId, key: &str) -> Option<f64> {
        let values: Vec<f64> = self
            .phases
            .get(&phase)?
            .iter()
            .filter_map(|value| {
                value
                    .get(key)
                    .or_else(|| value.get("frames").and_then(|f| f.get(key)))
                    .or_else(|| value.get("textures").and_then(|t| t.get(key)))
                    .and_then(Value::as_f64)
            })
            .collect();
        median(values)
    }

    fn run_metric(&self, pointer: &str) -> Option<f64> {
        self.root.pointer(pointer).and_then(Value::as_f64)
    }

    /// `false` when the file says it is non-representative, at run level or
    /// on any phase.
    fn representative(&self) -> bool {
        let flagged_false =
            |value: &Value| value.get("representative") == Some(&Value::Bool(false));
        !flagged_false(&self.root) && !self.phases.values().flatten().any(flagged_false)
    }

    fn paint_policy(&self) -> Option<&str> {
        let phase_policy = || {
            self.phases
                .values()
                .flatten()
                .find_map(|phase| phase.get("paintPolicy").and_then(Value::as_str))
        };
        self.root
            .get("paintPolicy")
            .and_then(Value::as_str)
            .or_else(phase_policy)
    }

    /// Whether the chrome layer was drawn. Rust-shell files from before the
    /// layer existed carry no field and drew none; other producers are
    /// unknown.
    fn chrome(&self) -> Option<bool> {
        self.recorded("chrome", Value::as_bool)
            .or_else(|| self.is_rust_shell().then_some(false))
    }

    /// Annotation count, with the same defaulting as [`Self::chrome`].
    fn annotations(&self) -> Option<u64> {
        self.recorded("annotations", Value::as_u64)
            .or_else(|| self.is_rust_shell().then_some(0))
    }

    /// A run-level or per-phase field, whichever the file has.
    fn recorded<T>(&self, key: &str, read: impl Fn(&Value) -> Option<T>) -> Option<T> {
        self.root.get(key).and_then(&read).or_else(|| {
            self.phases
                .values()
                .flatten()
                .find_map(|phase| phase.get(key).and_then(&read))
        })
    }

    /// Whether the file came from the Rust app: its report names the shell,
    /// and its JSON lines carry a paint policy on every phase.
    fn is_rust_shell(&self) -> bool {
        self.root.get("shell").and_then(Value::as_str) == Some("rust-cef")
            || self
                .phases
                .values()
                .flatten()
                .any(|phase| phase.get("paintPolicy").is_some())
    }

    fn notes(&self) -> impl Iterator<Item = &str> {
        self.root
            .get("notes")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
    }

    fn frame_ms(&self) -> Option<f64> {
        let phase_frame_ms = || {
            self.phases
                .values()
                .flatten()
                .find_map(|phase| phase.get("stepIntervalMs").and_then(Value::as_f64))
        };
        self.root
            .get("frameMs")
            .and_then(Value::as_f64)
            .or_else(phase_frame_ms)
    }
}

fn parse_json_lines(text: &str, name: &str) -> Result<Vec<Value>, BenchError> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line).map_err(|source| BenchError::Json {
                what: name.to_owned(),
                source,
            })
        })
        .collect()
}

fn phase_id(value: &Value) -> Option<Result<ProfileId, BenchError>> {
    let raw = value
        .get("phase")
        .or_else(|| value.get("profile"))?
        .as_str()?;
    Some(raw.parse())
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        f64::midpoint(values[mid - 1], values[mid])
    } else {
        values[mid]
    })
}

/// Lists every note either run carries, labelled by run.
fn write_notes(out: &mut String, a: &LoadedRun, b: &LoadedRun) {
    let notes: Vec<(&str, &str)> = [a, b]
        .into_iter()
        .flat_map(|run| run.notes().map(move |note| (run.label(), note)))
        .collect();
    if notes.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n## Notes\n");
    for (label, note) in notes {
        let _ = writeln!(out, "- {label}: {note}");
    }
}

/// Says how much canvas chrome each run drew per frame, and warns when the
/// two differ: the chrome layer costs frame time, so such runs are confounded.
fn write_chrome(out: &mut String, a: &LoadedRun, b: &LoadedRun) {
    let describe = |run: &LoadedRun| match (run.chrome(), run.annotations()) {
        (Some(true), annotations) => {
            format!(
                "chrome on, {} annotations",
                format_value(annotations.map(|n| n as f64))
            )
        }
        (Some(false), _) => "chrome off".to_owned(),
        (None, _) => "chrome unknown".to_owned(),
    };
    let (da, db) = (describe(a), describe(b));
    if a.chrome().is_none() && b.chrome().is_none() {
        return;
    }
    let (la, lb) = (a.label(), b.label());
    let _ = writeln!(out, "\nCanvas chrome: {la} {da}; {lb} {db}.");
    let chrome_differs = matches!((a.chrome(), b.chrome()), (Some(x), Some(y)) if x != y);
    let annotations_differ =
        matches!((a.annotations(), b.annotations()), (Some(x), Some(y)) if x != y);
    if chrome_differs || annotations_differ {
        let _ = writeln!(
            out,
            "\n> **Chrome load differs:** {la} ran {da}, {lb} ran {db}; each frame drew \
             a different amount of UI, so frame times are confounded."
        );
    }
}

fn format_value(value: Option<f64>) -> String {
    match value {
        None => "—".to_owned(),
        Some(v) if v.fract() == 0.0 => format!("{v:.0}"),
        Some(v) => format!("{v:.2}"),
    }
}

fn format_delta(a: Option<f64>, b: Option<f64>) -> (String, String) {
    let (Some(a), Some(b)) = (a, b) else {
        return (String::new(), String::new());
    };
    let delta = b - a;
    let percent = if a == 0.0 {
        String::new()
    } else {
        format!("{:+.1}%", delta / a.abs() * 100.0)
    };
    (format!("{delta:+.2}"), percent)
}

/// Renders the comparison of `a` against `b` (deltas are `b - a`).
pub fn compare_markdown(a: &LoadedRun, b: &LoadedRun) -> String {
    let mut out = String::new();
    let (la, lb) = (a.label(), b.label());
    let _ = writeln!(out, "# {la} vs {lb}\n");
    for run in [a, b] {
        if !run.representative() {
            let _ = writeln!(
                out,
                "> **Not representative:** {} includes CPU-copied or synthetic frames; \
                 exclude it from the verdict (ADR 0038).\n",
                run.label()
            );
        }
    }
    let (fa, fb) = (a.frame_ms(), b.frame_ms());
    let _ = writeln!(
        out,
        "Refresh budget (`frameMs`): {la} {}, {lb} {}.",
        format_value(fa),
        format_value(fb)
    );
    if let (Some(fa), Some(fb)) = (fa, fb)
        && (fa - fb).abs() > 0.5
    {
        let _ = writeln!(
            out,
            "\n> **Budgets differ:** runs were stepped at different refresh rates; \
             `longFrames` and fps are not comparable."
        );
    }
    if let (Some(pa), Some(pb)) = (a.paint_policy(), b.paint_policy())
        && pa != pb
    {
        let _ = writeln!(
            out,
            "\n> **Paint policies differ:** {la} ran `{pa}`, {lb} ran `{pb}`; the shells \
             painted different amounts per page, so frame times and memory are confounded."
        );
    }
    write_chrome(&mut out, a, b);
    let _ = writeln!(out, "\nDeltas are {lb} minus {la}.\n");

    let _ = writeln!(out, "## Frame timing per profile\n");
    let _ = writeln!(out, "| Profile | Metric | {la} | {lb} | Δ | Δ% |");
    let _ = writeln!(out, "|---|---|---:|---:|---:|---:|");
    for id in ProfileId::ALL {
        for metric in PHASE_METRICS {
            let (va, vb) = (
                a.phase_metric(id, metric.key),
                b.phase_metric(id, metric.key),
            );
            if va.is_none() && vb.is_none() {
                continue;
            }
            let (delta, percent) = format_delta(va, vb);
            let _ = writeln!(
                out,
                "| {id} | {} | {} | {} | {delta} | {percent} |",
                metric.label,
                format_value(va),
                format_value(vb)
            );
        }
    }

    let run_rows: Vec<String> = RUN_METRICS
        .iter()
        .filter_map(|metric| {
            let (va, vb) = (a.run_metric(metric.key), b.run_metric(metric.key));
            if va.is_none() && vb.is_none() {
                return None;
            }
            let (delta, percent) = format_delta(va, vb);
            Some(format!(
                "| {} | {} | {} | {delta} | {percent} |",
                metric.label,
                format_value(va),
                format_value(vb)
            ))
        })
        .collect();
    if !run_rows.is_empty() {
        let _ = writeln!(out, "\n## Whole run\n");
        let _ = writeln!(out, "| Metric | {la} | {lb} | Δ | Δ% |");
        let _ = writeln!(out, "|---|---:|---:|---:|---:|");
        for row in run_rows {
            let _ = writeln!(out, "{row}");
        }
        let _ = writeln!(
            out,
            "\nJudge memory by footprint: RSS misses IOSurface and GPU allocations and \
             double-counts pages shared between processes."
        );
    }
    write_notes(&mut out, a, b);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ELECTRON: &str = r#"{"shell":"electron","source":"chromium-trace","frameMs":8.33,
        "representative":true,
        "phases":[{"phase":"slow-pan","durationMs":2000,"draws":240,"drawFps":120,
            "meanFrameMs":8.33,"p50FrameMs":8.3,"p95FrameMs":9.0,"p99FrameMs":9.5,
            "maxFrameMs":10,"longFrames":0}],
        "memory":{"idle":{"rootPid":1,"processes":12,"rssMb":1500}}}"#;

    const RUST_JSONL: &str = r#"{"profile":"slow-pan","representative":true,"stepIntervalMs":8.33,"frames":{"draws":240,"drawFps":120,"meanFrameMs":8.33,"p50FrameMs":8.3,"p95FrameMs":8.5,"p99FrameMs":8.9,"maxFrameMs":9,"longFrames":0}}
{"profile":"slow-pan","representative":true,"stepIntervalMs":8.33,"frames":{"draws":240,"drawFps":120,"meanFrameMs":8.33,"p50FrameMs":8.3,"p95FrameMs":8.7,"p99FrameMs":8.9,"maxFrameMs":11,"longFrames":0}}
{"profile":"slow-pan","representative":true,"stepIntervalMs":8.33,"frames":{"draws":240,"drawFps":120,"meanFrameMs":8.33,"p50FrameMs":8.3,"p95FrameMs":8.6,"p99FrameMs":8.9,"maxFrameMs":10,"longFrames":0}}
"#;

    fn electron() -> LoadedRun {
        LoadedRun::parse(ELECTRON, "electron.json").unwrap()
    }

    fn rust() -> LoadedRun {
        LoadedRun::parse(RUST_JSONL, "rust.jsonl").unwrap()
    }

    #[test]
    fn run_report_label_comes_from_shell() {
        assert_eq!(electron().label(), "Electron");
    }

    #[test]
    fn json_lines_label_falls_back_to_file_name() {
        assert_eq!(rust().label(), "rust.jsonl");
    }

    #[test]
    fn repeated_profiles_reduce_to_median() {
        assert_eq!(
            rust().phase_metric(ProfileId::SlowPan, "p95FrameMs"),
            Some(8.6)
        );
    }

    #[test]
    fn nested_frames_metrics_are_found() {
        assert_eq!(
            rust().phase_metric(ProfileId::SlowPan, "maxFrameMs"),
            Some(10.0)
        );
    }

    #[test]
    fn lab_phase_array_with_aliases_loads() {
        let lab = r#"[{"phase":"fast-pan","draws":54,"drawFps":120,"meanFrameMs":8.33,
            "maxFrameMs":9.4,"longFrames":0,"framesReceived":3}]"#;
        let run = LoadedRun::parse(lab, "lab.json").unwrap();
        assert_eq!(
            run.phase_metric(ProfileId::FastDiagonalPan, "maxFrameMs"),
            Some(9.4)
        );
    }

    #[test]
    fn pan_zoom_run_response_is_rejected_with_guidance() {
        let response =
            r#"{"cancelled":false,"tracePath":"/x.json","fileName":"x.json","frameMs":8.33}"#;
        let error = LoadedRun::parse(response, "run.json").unwrap_err();
        assert!(error.to_string().contains("electron-trace"));
    }

    #[test]
    fn unknown_profile_id_is_an_error() {
        assert!(LoadedRun::parse(r#"[{"phase":"wiggle"}]"#, "x").is_err());
    }

    #[test]
    fn markdown_has_row_with_both_values_and_delta() {
        let table = compare_markdown(&electron(), &rust());
        assert!(
            table.contains("| slow-pan | p95 ms | 9 | 8.60 | -0.40 | -4.4% |"),
            "{table}"
        );
    }

    #[test]
    fn markdown_shows_dash_for_one_sided_run_metric() {
        let table = compare_markdown(&electron(), &rust());
        assert!(
            table.contains("| RSS idle (MB) | 1500 | — |  |  |"),
            "{table}"
        );
    }

    #[test]
    fn markdown_flags_differing_paint_policies() {
        let lod = ELECTRON.replace(
            r#""representative":true,"#,
            r#""representative":true,"paintPolicy":"electron-lod","#,
        );
        let full = lod.replace("electron-lod", "full-rate");
        let (a, b) = (
            LoadedRun::parse(&lod, "a.json").unwrap(),
            LoadedRun::parse(&full, "b.json").unwrap(),
        );
        assert!(compare_markdown(&a, &b).contains("Paint policies differ"));
    }

    #[test]
    fn markdown_flags_differing_chrome_load() {
        // An older Rust-app file: its lines carry a paint policy, no chrome.
        let plain = RUST_JSONL.replace(
            r#""representative":true"#,
            r#""representative":true,"paintPolicy":"electron-lod""#,
        );
        let loaded = RUST_JSONL.replace(
            r#""representative":true"#,
            r#""representative":true,"paintPolicy":"electron-lod","chrome":true,"annotations":40"#,
        );
        let (a, b) = (
            LoadedRun::parse(&plain, "a.jsonl").unwrap(),
            LoadedRun::parse(&loaded, "b.jsonl").unwrap(),
        );
        let table = compare_markdown(&a, &b);
        assert!(table.contains("Chrome load differs"), "{table}");
    }

    #[test]
    fn markdown_stays_quiet_when_chrome_load_matches() {
        let loaded = RUST_JSONL.replace(
            r#""representative":true"#,
            r#""representative":true,"chrome":true,"annotations":40"#,
        );
        let run = LoadedRun::parse(&loaded, "a.jsonl").unwrap();
        assert!(!compare_markdown(&run, &run).contains("Chrome load differs"));
    }

    #[test]
    fn electron_runs_have_unknown_chrome() {
        assert_eq!(electron().chrome(), None);
    }

    #[test]
    fn markdown_lists_run_notes() {
        let noted = ELECTRON.replace(
            r#""representative":true,"#,
            r#""representative":true,"notes":["presents unverified"],"#,
        );
        let run = LoadedRun::parse(&noted, "a.json").unwrap();
        assert!(compare_markdown(&run, &rust()).contains("presents unverified"));
    }

    #[test]
    fn json_lines_skip_the_session_latency_line() {
        let text = format!("{RUST_JSONL}{}\n", r#"{"inputLatency":{"samples":1}}"#);
        assert!(LoadedRun::parse(&text, "rust.jsonl").is_ok());
    }

    #[test]
    fn markdown_flags_non_representative_runs() {
        let synthetic = RUST_JSONL.replace("\"representative\":true", "\"representative\":false");
        let run = LoadedRun::parse(&synthetic, "synthetic.jsonl").unwrap();
        assert!(compare_markdown(&electron(), &run).contains("Not representative"));
    }

    #[test]
    fn markdown_omits_metrics_neither_side_has() {
        let table = compare_markdown(&electron(), &rust());
        assert!(!table.contains("framesDroppedForPoolPressure"));
    }

    #[test]
    fn median_of_even_count_is_midpoint() {
        assert_eq!(median(vec![1.0, 3.0, 2.0, 4.0]), Some(2.5));
    }
}
