//! Run-level measurements taken outside the run (memory samples from
//! `specular-bench rss`, `/perf/page-hosts` snapshots), attached to a report
//! from files named on the command line.

use std::{fs, path::Path};

use anyhow::Context as _;
use serde::de::DeserializeOwned;
use specular_bench::{MemoryReport, MemorySample, PageHostsSnapshot, RunReport};

use crate::cli::Args;

/// Reads and parses a JSON file.
pub(crate) fn read_json<T: DeserializeOwned>(path: &str) -> anyhow::Result<T> {
    let text = fs::read_to_string(Path::new(path)).with_context(|| format!("reading {path}"))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {path}"))
}

/// Applies `--fixture`, `--pages`, `--memory-{idle,end,peak}` and
/// `--page-hosts-{before,after}` to `report`.
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

    if let Some(after) = args.flag("page-hosts-after") {
        let after = read_json::<PageHostsSnapshot>(after)?.total();
        let textures = if let Some(before) = args.flag("page-hosts-before") {
            after.since(read_json::<PageHostsSnapshot>(before)?.total())
        } else {
            report.notes.push(
                "texture counters are cumulative since each page host was created \
                 (no --page-hosts-before snapshot)"
                    .to_owned(),
            );
            after
        };
        report.textures = Some(textures);
        report.notes.push(
            "maxOutstandingTextures is the per-page peak over each page host's life, \
             not just this run"
                .to_owned(),
        );
    }
    Ok(())
}
