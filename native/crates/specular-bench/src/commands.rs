//! One function per subcommand.

use std::{fs, time::Duration};

use anyhow::{Context as _, bail};
use serde::{Deserialize, Serialize};
use specular_bench::{
    BenchLine, LoadedRun, MemorySample, PaintPolicy, PeakSampler, ProfileId, RunReport,
    STEP_INTERVAL, Shell, build_steps, compare_markdown,
    electron_trace::{DEFAULT_THREAD, PRESENT_EVENT, phases_from_trace, present_count_warning},
    sample_process_tree,
};

use crate::{
    attach::{attach_run_extras, read_json},
    cli::Args,
};

/// Bursts closer than this belong to one phase: under the test's 250 ms
/// phase gap, with room for timer slop on either side.
const DEFAULT_GAP: Duration = Duration::from_millis(200);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlannedProfile {
    id: ProfileId,
    label: &'static str,
    duration_ms: u128,
    steps: usize,
}

/// `plan`: the selected profiles and their step counts.
pub(crate) fn plan(args: &Args) -> anyhow::Result<String> {
    let interval = args.millis("frame-ms")?.unwrap_or(STEP_INTERVAL);
    let plan: Vec<PlannedProfile> = args
        .profiles()?
        .iter()
        .map(|profile| PlannedProfile {
            id: profile.id,
            label: profile.label,
            duration_ms: profile.duration.as_millis(),
            steps: build_steps(profile, interval).len(),
        })
        .collect();
    Ok(serde_json::to_string_pretty(&plan)?)
}

/// `compare a b`: markdown table of `b` against `a`.
pub(crate) fn compare(args: &Args) -> anyhow::Result<String> {
    let (Some(a), Some(b)) = (args.positional(0), args.positional(1)) else {
        bail!("usage: specular-bench compare <baseline.json> <candidate.json>");
    };
    let load = |path: &str| -> anyhow::Result<LoadedRun> {
        let text = fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
        Ok(LoadedRun::parse(&text, path)?)
    };
    Ok(compare_markdown(&load(a)?, &load(b)?))
}

/// The parts of a `/perf/pan-zoom/run` response the converter needs.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PanZoomRunResponse {
    trace_path: String,
    #[serde(default)]
    frame_ms: Option<f64>,
    #[serde(default)]
    cancelled: bool,
}

/// `electron-trace`: an Electron pan/zoom trace reduced to a report.
pub(crate) fn electron_trace(args: &Args) -> anyhow::Result<String> {
    let response: Option<PanZoomRunResponse> = args.flag("response").map(read_json).transpose()?;
    if response.as_ref().is_some_and(|r| r.cancelled) {
        bail!("the pan/zoom run was cancelled; its trace covers only some profiles");
    }
    let trace_path = args
        .positional(0)
        .map(str::to_owned)
        .or_else(|| response.as_ref().map(|r| r.trace_path.clone()))
        .context("usage: specular-bench electron-trace <trace.json> | --response run.json")?;
    let mut notes = Vec::new();
    let frame_ms = match (
        args.parsed::<f64>("frame-ms")?,
        response.as_ref().and_then(|r| r.frame_ms),
    ) {
        (Some(ms), _) | (None, Some(ms)) => ms,
        (None, None) => {
            notes.push("frameMs assumed 8.33 (120 Hz); pass --frame-ms or --response".to_owned());
            1_000.0 / 120.0
        }
    };
    let budget = Duration::from_secs_f64(frame_ms / 1_000.0);
    let thread = args.flag("thread").unwrap_or(DEFAULT_THREAD);
    let gap = args.millis("gap-ms")?.unwrap_or(DEFAULT_GAP);
    let profiles = args.profiles()?;

    let trace = fs::read_to_string(&trace_path).with_context(|| format!("reading {trace_path}"))?;
    let (timeline, phases) = phases_from_trace(&trace, thread, &profiles, budget, gap)?;
    notes.push(format!(
        "presents are `{PRESENT_EVENT}` slices on {} ({} total); phases recovered from \
         idle gaps over {} ms",
        timeline.thread,
        timeline.presents_us.len(),
        gap.as_millis()
    ));
    let warnings: Vec<String> = profiles
        .iter()
        .zip(&phases)
        .filter_map(|(profile, phase)| present_count_warning(profile, phase, budget))
        .collect();
    if warnings.is_empty() {
        notes.push(
            "present counts match one window (about steps + 1 per phase); confirm once \
             against a real ADR 0038 trace before trusting them"
                .to_owned(),
        );
    }
    notes.extend(warnings);
    let mut report = RunReport {
        shell: Shell::Electron,
        source: "chromium-trace".to_owned(),
        fixture: None,
        page_count: None,
        frame_ms,
        representative: true,
        paint_policy: Some(
            args.parsed::<PaintPolicy>("paint-policy")?
                .unwrap_or(PaintPolicy::ElectronLod),
        ),
        chrome: None,
        annotations: None,
        phases,
        textures: None,
        memory: None,
        input_latency: None,
        notes,
    };
    attach_run_extras(&mut report, args)?;
    Ok(serde_json::to_string_pretty(&report)?)
}

/// `assemble`: the app's JSON lines folded into one report.
pub(crate) fn assemble(args: &Args) -> anyhow::Result<String> {
    let path = args
        .positional(0)
        .context("usage: specular-bench assemble <bench.jsonl>")?;
    let text = fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    let mut report = assemble_lines(&text).with_context(|| format!("assembling {path}"))?;
    attach_run_extras(&mut report, args)?;
    Ok(serde_json::to_string_pretty(&report)?)
}

/// Folds `specular-app` output lines into a report. Profile lines must agree
/// on source, page count, step interval, paint policy and chrome load.
fn assemble_lines(text: &str) -> anyhow::Result<RunReport> {
    let mut report = RunReport {
        shell: Shell::RustCef,
        source: String::new(),
        fixture: None,
        page_count: None,
        frame_ms: 0.0,
        representative: true,
        paint_policy: None,
        chrome: None,
        annotations: None,
        phases: Vec::new(),
        textures: None,
        memory: None,
        input_latency: None,
        notes: Vec::new(),
    };
    for (index, line) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let line_no = index + 1;
        match serde_json::from_str::<BenchLine>(line).with_context(|| format!("line {line_no}"))? {
            BenchLine::Profile(profile) => {
                if report.phases.is_empty() {
                    report.source = profile.source;
                    report.page_count = Some(profile.pages);
                    report.frame_ms = profile.step_interval_ms;
                    report.paint_policy = Some(profile.paint_policy);
                    report.chrome = Some(profile.chrome);
                    report.annotations = Some(profile.annotations);
                } else if report.source != profile.source
                    || report.page_count != Some(profile.pages)
                    || report.paint_policy != Some(profile.paint_policy)
                    || report.chrome != Some(profile.chrome)
                    || report.annotations != Some(profile.annotations)
                    || (report.frame_ms - profile.step_interval_ms).abs() > 0.5
                {
                    bail!("line {line_no} comes from a different run configuration");
                }
                report.representative &= profile.representative;
                report.phases.push(profile.phase);
            }
            BenchLine::InputLatency(latency) => report.input_latency = Some(latency.input_latency),
        }
    }
    if report.phases.is_empty() {
        bail!("no profile results");
    }
    Ok(report)
}

/// `rss --pid N [--peak-ms D]`: tree RSS now, or the peak over `D`.
pub(crate) fn rss(args: &Args) -> anyhow::Result<String> {
    let pid = args
        .parsed::<u32>("pid")?
        .context("usage: specular-bench rss --pid <root pid> [--peak-ms <ms>]")?;
    let sample: MemorySample = match args.millis("peak-ms")? {
        Some(window) => {
            let sampler = PeakSampler::spawn(pid, Duration::from_millis(100));
            std::thread::sleep(window);
            sampler
                .finish()
                .with_context(|| format!("no sample of process {pid} succeeded"))?
        }
        None => sample_process_tree(pid)?,
    };
    Ok(serde_json::to_string_pretty(&sample)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROFILE: &str = r#"{"phase":"slow-zoom","durationMs":2000,"draws":240,"drawFps":120,
        "meanFrameMs":8.33,"p50FrameMs":8.3,"p95FrameMs":8.6,"p99FrameMs":8.9,"maxFrameMs":9.1,
        "longFrames":0,"label":"Slow zoom","source":"cef","pages":9,"representative":true,
        "stepIntervalMs":8.33,"maxPaintToSubmitMs":null,"paintPolicy":"electron-lod"}"#;

    fn one_line(json: &str) -> String {
        json.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn profile_line_becomes_phase_report() {
        let report = assemble_lines(&one_line(PROFILE)).unwrap();
        assert_eq!(
            (report.phases[0].phase, report.phases[0].frames.frames),
            (ProfileId::SlowZoom, 240)
        );
    }

    #[test]
    fn input_latency_line_fills_run_latency() {
        let text = format!(
            "{}\n{}",
            one_line(PROFILE),
            r#"{"inputLatency":{"samples":4,"unresolved":0,"meanMs":20,"p50Ms":18,"p95Ms":30,"maxMs":31}}"#
        );
        let report = assemble_lines(&text).unwrap();
        assert_eq!(report.input_latency.map(|l| l.samples), Some(4));
    }

    #[test]
    fn line_with_renamed_field_is_rejected() {
        let renamed = one_line(PROFILE).replace("stepIntervalMs", "stepMs");
        assert!(assemble_lines(&renamed).is_err());
    }

    #[test]
    fn lines_from_different_paint_policies_are_rejected() {
        let other = one_line(PROFILE).replace("electron-lod", "full-rate");
        let text = format!("{}\n{other}", one_line(PROFILE));
        assert!(assemble_lines(&text).is_err());
    }

    #[test]
    fn old_lines_assemble_as_no_chrome() {
        let report = assemble_lines(&one_line(PROFILE)).unwrap();
        assert_eq!((report.chrome, report.annotations), (Some(false), Some(0)));
    }

    #[test]
    fn lines_with_different_annotation_counts_are_rejected() {
        let other = one_line(PROFILE).replace(r#""pages":9,"#, r#""pages":9,"annotations":5,"#);
        let text = format!("{}\n{other}", one_line(PROFILE));
        assert!(assemble_lines(&text).is_err());
    }

    #[test]
    fn chrome_fields_carry_into_the_report() {
        let line = one_line(PROFILE).replace(
            r#""pages":9,"#,
            r#""pages":9,"chrome":true,"annotations":40,"maxShapesDrawn":52,"#,
        );
        let report = assemble_lines(&line).unwrap();
        assert_eq!(
            (
                report.chrome,
                report.annotations,
                report.phases[0].max_shapes_drawn
            ),
            (Some(true), Some(40), Some(52))
        );
    }

    #[test]
    fn file_without_profiles_is_rejected() {
        let latency = r#"{"inputLatency":{"samples":0,"meanMs":0,"p50Ms":0,"p95Ms":0,"maxMs":0}}"#;
        assert!(assemble_lines(latency).is_err());
    }
}
