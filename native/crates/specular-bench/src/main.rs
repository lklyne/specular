//! `specular-bench`: gesture plans, process-tree memory sampling, and the
//! app's bench lines folded into a results file.

mod attach;
mod cli;
mod commands;

use anyhow::bail;

use crate::cli::Args;

const USAGE: &str = "\
usage: specular-bench <command> [args]

  plan            [--profiles a,b] [--duration-ms N] [--frame-ms N]
                  Print the gesture steps each profile expands to.
  assemble        <bench.jsonl> [--fixture NAME] [--pages N] [--memory-* f]
                  Fold the Rust app's JSON lines (--bench profiles, and an
                  interactive session's inputLatency line) into a report.
  rss             --pid N [--peak-ms N | --per-process true]
                  Footprint (macOS) and RSS of a process and its descendants.
  resize-report   <ledger.jsonl>...
                  One JSON line a resize ledger (SPECULAR_RESIZE_TRACE),
                  then a table of them all.
";

fn main() -> anyhow::Result<()> {
    let mut raw = std::env::args().skip(1);
    let command = raw.next().unwrap_or_else(|| "plan".to_owned());
    let args = Args::parse(raw)?;
    let output = match command.as_str() {
        "plan" => commands::plan(&args)?,
        "assemble" => commands::assemble(&args)?,
        "rss" => commands::rss(&args)?,
        "resize-report" => commands::resize_report(&args)?,
        "help" | "-h" | "--help" => USAGE.to_owned(),
        other => bail!("unknown command `{other}`\n\n{USAGE}"),
    };
    println!("{output}");
    Ok(())
}
