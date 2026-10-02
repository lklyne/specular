//! One function per subcommand.

use std::{fs, time::Duration};

use anyhow::{Context as _, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specular_bench::{
    FrameSummary, LoadedRun, MemorySample, PeakSampler, PhaseReport, ProfileId, RunReport,
    STEP_INTERVAL, Shell, build_steps, compare_markdown,
    electron_trace::{DEFAULT_THREAD, PRESENT_EVENT, phases_from_trace},
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
    let mut report = RunReport {
        shell: Shell::Electron,
        source: "chromium-trace".to_owned(),
        fixture: None,
        page_count: None,
        frame_ms,
        representative: true,
        phases,
        textures: None,
        memory: None,
        input_latency: None,
        gesture_latency: None,
        notes,
    };
    attach_run_extras(&mut report, args)?;
    Ok(serde_json::to_string_pretty(&report)?)
}

/// `assemble`: the app's `--bench` JSON lines folded into one report.
pub(crate) fn assemble(args: &Args) -> anyhow::Result<String> {
    let path = args
        .positional(0)
        .context("usage: specular-bench assemble <bench.jsonl>")?;
    let text = fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    let mut report = RunReport {
        shell: Shell::RustCef,
        source: String::new(),
        fixture: None,
        page_count: None,
        frame_ms: 0.0,
        representative: true,
        phases: Vec::new(),
        textures: None,
        memory: None,
        input_latency: None,
        gesture_latency: None,
        notes: Vec::new(),
    };
    for (index, line) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let value: Value =
            serde_json::from_str(line).with_context(|| format!("{path}:{}", index + 1))?;
        report
            .phases
            .push(phase_from_line(&value).with_context(|| format!("{path}:{}", index + 1))?);
        report.representative &= value.get("representative") != Some(&Value::Bool(false));
        if let Some(source) = value.get("source").and_then(Value::as_str) {
            source.clone_into(&mut report.source);
        }
        if let Some(ms) = value.get("stepIntervalMs").and_then(Value::as_f64) {
            report.frame_ms = ms;
        }
        if let Some(pages) = value.get("pages").and_then(Value::as_u64) {
            report.page_count = usize::try_from(pages).ok();
        }
    }
    if report.phases.is_empty() {
        bail!("{path} holds no profile results");
    }
    attach_run_extras(&mut report, args)?;
    Ok(serde_json::to_string_pretty(&report)?)
}

fn phase_from_line(value: &Value) -> anyhow::Result<PhaseReport> {
    let id = value
        .get("phase")
        .or_else(|| value.get("profile"))
        .and_then(Value::as_str)
        .context("no `phase` or `profile` field")?
        .parse::<ProfileId>()?;
    let frames: FrameSummary = serde_json::from_value(
        value
            .get("frames")
            .cloned()
            .unwrap_or_else(|| value.clone()),
    )
    .context("no frame summary")?;
    Ok(PhaseReport {
        phase: id,
        duration_ms: value
            .get("durationMs")
            .and_then(Value::as_f64)
            .unwrap_or_default(),
        frames,
        frames_received: value.get("framesReceived").and_then(Value::as_u64),
        draws_without_texture: value.get("drawsWithoutTexture").and_then(Value::as_u64),
        textures: value
            .get("textures")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?,
    })
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

    #[test]
    fn app_bench_line_becomes_phase_report() {
        let line: Value = serde_json::from_str(
            r#"{"profile":"slow-zoom","label":"Slow zoom","source":"cef","pages":9,
                "representative":true,"stepIntervalMs":8.33,
                "frames":{"draws":240,"drawFps":120,"meanFrameMs":8.33,"p50FrameMs":8.3,
                  "p95FrameMs":8.6,"p99FrameMs":8.9,"maxFrameMs":9.1,"longFrames":0},
                "maxPaintToSubmitMs":null,"inputToPresentMs":null}"#,
        )
        .unwrap();
        let phase = phase_from_line(&line).unwrap();
        assert_eq!(
            (phase.phase, phase.frames.frames),
            (ProfileId::SlowZoom, 240)
        );
    }

    #[test]
    fn line_without_profile_is_rejected() {
        let line: Value = serde_json::from_str(r#"{"frames":{}}"#).unwrap();
        assert!(phase_from_line(&line).is_err());
    }
}
