//! The summaries of several runs as one table, a run a row.

use std::fmt::Write as _;

use super::{Spread, Summary};

fn spread(spread: &Spread) -> String {
    if spread.n == 0 {
        return "-".to_owned();
    }
    format!("{:.1}/{:.1}/{:.1}", spread.p50, spread.p95, spread.max)
}

/// The middle and the worst of some times, and how many are missing.
fn middle_and_worst(times: impl Iterator<Item = Option<f64>>) -> String {
    let all: Vec<Option<f64>> = times.collect();
    let mut known: Vec<f64> = all.iter().flatten().copied().collect();
    known.sort_by(f64::total_cmp);
    let middle = known.get(known.len() / 2).copied().unwrap_or(0.0);
    let worst = known.last().copied().unwrap_or(0.0);
    format!("{middle:.0}/{worst:.0} ({} never)", all.len() - known.len())
}

/// How many switches, their asks on average, and the median and worst time
/// to a frame at the CSS size and then at the texel size too.
fn switches(summary: &Summary) -> String {
    let switches = &summary.switches;
    if switches.is_empty() || summary.live_ms > 0.0 {
        return "-".to_owned();
    }
    let asks: usize = switches.iter().map(|switch| switch.asks).sum();
    format!(
        "{}x {:.1} asks, css {}, texels {}",
        switches.len(),
        asks as f64 / switches.len() as f64,
        middle_and_worst(switches.iter().map(|switch| switch.css_ms)),
        middle_and_worst(switches.iter().map(|switch| switch.settle_ms)),
    )
}

/// A table of `summaries`. Times are milliseconds, and three numbers with
/// slashes are the median, the 95th percentile and the longest.
pub fn table(summaries: &[Summary]) -> String {
    let head = [
        "run",
        "live ms",
        "steps",
        "frames (fps)",
        "tick Hz",
        "step>frame",
        "no frame",
        "asks/reqs",
        "paints (fit)",
        "unanswered",
        "ask>read",
        "read>paint",
        "paint>frame",
        "stale",
        "gap css",
        "pumps (ms, max)",
        "drawable",
        "imports (ms)",
        "busy live/all",
        "busy max",
        "switches",
    ];
    let mut out = format!("| {} |\n", head.join(" | "));
    let _ = writeln!(out, "|{}", "---|".repeat(head.len()));
    for run in summaries {
        let page = &run.page;
        let cells = [
            run.run.clone(),
            format!("{:.0}", run.live_ms),
            run.win_steps.to_string(),
            format!("{} ({:.0})", run.live_frames, run.live_fps),
            format!("{:.0}", run.live_tick_hz),
            spread(&run.win_to_frame),
            run.win_unmatched.to_string(),
            format!("{}/{}", page.asks, page.requests),
            format!("{} ({})", page.paints, page.paints_matching),
            page.unanswered.to_string(),
            spread(&page.ask_to_view_rect),
            spread(&page.view_rect_to_paint),
            spread(&page.paint_to_frame),
            format!("{:.0}% of {}", run.stale_share * 100.0, run.page_frames),
            run.widest_gap_css.to_string(),
            format!(
                "{} ({:.0}, {:.1})",
                run.pumps, run.pump_total_ms, run.pump_max_ms
            ),
            format!("{:.1}/{:.1}", run.acquire.p50, run.acquire.max),
            format!("{} ({:.1})", run.import_misses, run.import_total_ms),
            format!(
                "{:.0}%/{:.0}%",
                run.live_busy_share * 100.0,
                run.busy_share * 100.0
            ),
            format!("{:.1}", run.busy_max_ms),
            switches(run),
        ];
        let _ = writeln!(out, "| {} |", cells.join(" | "));
    }
    out
}
