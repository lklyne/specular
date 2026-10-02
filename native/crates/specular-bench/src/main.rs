//! `specular-bench`: gesture plans, Electron trace conversion, process-tree
//! memory sampling, and side-by-side comparison of results files.

mod attach;
mod cli;
mod commands;

use anyhow::bail;

use crate::cli::Args;

const USAGE: &str = "\
usage: specular-bench <command> [args]

  plan            [--profiles a,b] [--duration-ms N] [--frame-ms N]
                  Print the gesture steps each profile expands to.
  compare         <baseline.json> <candidate.json>
                  Markdown table of candidate against baseline.
  electron-trace  <trace.json> | --response run.json
                  [--frame-ms N] [--profiles a,b] [--duration-ms N]
                  [--gap-ms 200] [--thread VizCompositorThread]
                  [--paint-policy electron-lod|full-rate]
                  [--fixture NAME] [--pages N]
                  [--memory-idle f] [--memory-end f] [--memory-peak f]
                  [--page-hosts-before f] [--page-hosts-after f]
                  Reduce an Electron /perf/pan-zoom/run trace to a report.
  assemble        <bench.jsonl> [--fixture NAME] [--pages N] [--memory-* f]
                  Fold the Rust app's JSON lines (--bench profiles, and an
                  interactive session's inputLatency line) into a report.
  rss             --pid N [--peak-ms N]
                  Footprint (macOS) and RSS of a process and its descendants.
";

fn main() -> anyhow::Result<()> {
    let mut raw = std::env::args().skip(1);
    let command = raw.next().unwrap_or_else(|| "plan".to_owned());
    let args = Args::parse(raw)?;
    let output = match command.as_str() {
        "plan" => commands::plan(&args)?,
        "compare" => commands::compare(&args)?,
        "electron-trace" => commands::electron_trace(&args)?,
        "assemble" => commands::assemble(&args)?,
        "rss" => commands::rss(&args)?,
        "help" | "-h" | "--help" => USAGE.to_owned(),
        other => bail!("unknown command `{other}`\n\n{USAGE}"),
    };
    println!("{output}");
    Ok(())
}
