//! One function per subcommand.

use std::{fs, time::Duration};

use anyhow::{Context as _, bail};
use serde::Serialize;
use specular_bench::{
    BenchLine, MemorySample, PeakSampler, ProfileId, RunReport, STEP_INTERVAL, build_steps,
    sample_breakdown, sample_process_tree,
};

use crate::{attach::attach_run_extras, cli::Args};

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

/// `rss --pid N [--peak-ms D | --per-process]`: tree RSS now, the peak over
/// `D`, or one row a process.
pub(crate) fn rss(args: &Args) -> anyhow::Result<String> {
    let pid = args.parsed::<u32>("pid")?.context(
        "usage: specular-bench rss --pid <root pid> [--peak-ms <ms> | --per-process true]",
    )?;
    if args.parsed::<bool>("per-process")?.unwrap_or(false) {
        return Ok(serde_json::to_string_pretty(&sample_breakdown(pid)?)?);
    }
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

/// `resize-report <ledger.jsonl>...`: a JSON line for each resize ledger,
/// then one table of them.
pub(crate) fn resize_report(args: &Args) -> anyhow::Result<String> {
    use specular_bench::resize_report::{parse, summarize, table};

    if args.positionals().is_empty() {
        anyhow::bail!("usage: specular-bench resize-report <ledger.jsonl>...");
    }
    let mut summaries = Vec::new();
    for path in args.positionals() {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
        let events = parse(&text).with_context(|| format!("{path} is not a resize ledger"))?;
        let name = std::path::Path::new(path).file_stem();
        let name = name.map_or_else(|| path.clone(), |stem| stem.to_string_lossy().into_owned());
        summaries.push(summarize(&name, &events));
    }
    let mut out = String::new();
    for summary in &summaries {
        out.push_str(&serde_json::to_string(summary)?);
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&table(&summaries));
    Ok(out)
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
}
