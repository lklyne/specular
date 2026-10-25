//! The scenario script: one step a line, run the same by the headless
//! runner (`--script`) and by the Kit window (`SPECULAR_SHELL_SCRIPT_FILE`).
//!
//! ```text
//! # canvas: ../kitchen-sink.canvas
//! # select the sticky, then start a marquee
//! camera 0,0,1
//! click 240 180
//! expect selection sticky-1
//! press 40 40
//! drag-to 600 420
//! release
//! expect count entities 12
//! undo-to-start
//! ```
//!
//! The first line may name the canvas the run opens, relative to the
//! script's folder. `given` lines (see [`given`]) before the first step
//! describe the starting document instead. A blank line or one starting
//! with `#` is skipped. Positions are logical pixels in the viewport.
//! A step only a window can do ([`WindowStep`]) is skipped by a headless
//! run, which says so when it ends.

mod check;
mod expect;
mod facts_canvas;
mod given;
mod saved;
mod state;
mod step;
mod window;
mod words;

use std::path::Path;

use anyhow::{Context as _, bail};
use specular_core::Camera;
use specular_doc::Document;

pub(crate) use self::check::is_markdown;
pub use self::check::{Checks, saved_text};
pub use self::expect::{Counted, EntityFact, Expect, Reference, SavedFact};
pub use self::facts_canvas::{CanvasFact, CanvasStep, Query as CanvasQuery, check_sent};
pub use self::given::Given;
pub use self::state::dump;
pub use self::step::Step;
pub use self::window::WindowStep;
pub(crate) use self::words::{camera, theme_named};

/// Where the camera is put.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraArg {
    /// Zoom to fit the whole document, never above 100%.
    Fit,
    /// This pan and zoom.
    At(Camera),
}

/// One step and where it was written.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The line number, from 1.
    pub number: usize,
    /// The line as written, trimmed.
    pub text: String,
    /// What it does.
    pub step: Step,
}

/// A whole script.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Script {
    /// The canvas the `# canvas:` line names, as written.
    pub canvas: Option<String>,
    /// The `given` lines, in order.
    pub given: Vec<Given>,
    /// The steps, in order.
    pub steps: Vec<Line>,
}

impl Script {
    /// The script in `text`, or the first line that is not a step.
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let mut script = Self::default();
        for (index, raw) in text.lines().enumerate() {
            let number = index + 1;
            let line = raw.trim();
            if let Some(canvas) = line.strip_prefix("# canvas:") {
                if number == 1 {
                    script.canvas = Some(canvas.trim().to_owned());
                }
                continue;
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let context = || format!("line {number}: `{line}`");
            if let Some(rest) = line.strip_prefix("given ") {
                if !script.steps.is_empty() {
                    bail!("line {number}: `given` lines come before the first step");
                }
                script.given.push(given::parse(rest).with_context(context)?);
                continue;
            }
            let step = step::step(raw.trim_start()).with_context(context)?;
            script.steps.push(Line {
                number,
                text: line.to_owned(),
                step,
            });
        }
        Ok(script)
    }

    /// The script in the file at `path`.
    pub fn read(path: &Path) -> anyhow::Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("in {}", path.display()))
    }

    /// The document the `given` lines describe, if there are any.
    pub fn given_document(&self) -> anyhow::Result<Option<Document>> {
        if self.given.is_empty() {
            return Ok(None);
        }
        given::document(&self.given).map(Some)
    }
}
