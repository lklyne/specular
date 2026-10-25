//! Run-level measurements taken outside the run (memory samples from
//! `specular-bench rss`), attached to a report from files named on the
//! command line.

use std::{fs, path::Path};

use anyhow::Context as _;
use serde::de::DeserializeOwned;
use specular_bench::{MemoryReport, MemorySample, RunReport};

use crate::cli::Args;

/// Reads and parses a JSON file.
fn read_json<T: DeserializeOwned>(path: &str) -> anyhow::Result<T> {
    let text = fs::read_to_string(Path::new(path)).with_context(|| format!("reading {path}"))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {path}"))
}

/// Applies `--fixture`, `--pages` and `--memory-{idle,end,peak}` to
/// `report`.
pub(crate) fn attach_run_extras(report: &mut RunReport, args: &Args) -> anyhow::Result<()> {
    if let Some(fixture) = args.flag("fixture") {
        report.fixture = Some(fixture.to_owned());
    }
    if let Some(pages) = args.parsed::<usize>("pages")? {
        report.page_count = Some(pages);
    }

    let sample = |flag: &str| -> anyhow::Result<Option<MemorySample>> {
        args.flag(flag).map(read_json).transpose()
    };
    let memory = MemoryReport {
        idle: sample("memory-idle")?,
        end: sample("memory-end")?,
        peak: sample("memory-peak")?,
    };
    if memory != MemoryReport::default() {
        report.memory = Some(memory);
    }

    Ok(())
}
