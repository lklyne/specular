//! The markdown table two loaded runs are compared in.

use std::fmt::Write as _;

use super::{LoadedRun, PHASE_METRICS, RUN_METRICS};
use crate::ProfileId;

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
