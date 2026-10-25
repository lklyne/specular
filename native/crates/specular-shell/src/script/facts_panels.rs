//! Facts about what the panels, the text editor, comments, chat and pages
//! show, and the steps that play the shell's and the agent's side of them.
//!
//! Each of these is read as lines of text, a topic at a time, and an
//! `expect` tests the lines:
//!
//! ```text
//! expect chat has bubble agent "Made it blue."   # a line holds this text
//! expect chat lacks run                         # no line holds it
//! expect controls is page.back disabled "Back"  # a line is exactly this
//! expect comments count 2 mark                  # this many lines hold it
//! ```
//!
//! The topics are `chat`, `comments`, `controls`, `editor`, `effects`,
//! `inspect`, `menubar`, `onboarding`, `pages`, `palette`, `pointer`, `repos`,
//! `settings`, `sidebar` and `tabs` (see [`lines`]). A `state` step writes
//! every line of every topic, each as the `expect .. has` that holds it.
//! `effects` are the ones the step before asked of the shell (the same ones
//! `expect sent` reads), which only a headless run sees.

mod lines;
mod steps;

use std::fmt::Write as _;

use anyhow::{bail, ensure};
use specular_interact::{App, Effect};

pub use self::steps::PanelStep;
pub(super) use self::steps::parse as parse_step;

/// A topic whose lines a [`PanelFact`] tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    /// The right panel: threads, transcript, composer.
    Chat,
    /// Comment marks, the draft, the focus, the outlines, each comment.
    Comments,
    /// Every control the models hold, with its state.
    Controls,
    /// The caret, the selection and the composition of the text edited.
    Editor,
    /// What the step before asked of the shell.
    Effects,
    /// The inspect tool's outline and popover, and the pick.
    Inspect,
    /// The menu bar's items.
    Menubar,
    /// The first-run view.
    Onboarding,
    /// What each page has reported, and how it is marked.
    Pages,
    /// The command palette's items.
    Palette,
    /// What the pointer is over.
    Pointer,
    /// The connected repos.
    Repos,
    /// The settings window.
    Settings,
    /// The left sidebar's rows.
    Sidebar,
    /// The tab row, the lens and the eye.
    Tabs,
}

const TOPICS: [(&str, Topic); 15] = [
    ("chat", Topic::Chat),
    ("comments", Topic::Comments),
    ("controls", Topic::Controls),
    ("editor", Topic::Editor),
    ("effects", Topic::Effects),
    ("inspect", Topic::Inspect),
    ("menubar", Topic::Menubar),
    ("onboarding", Topic::Onboarding),
    ("pages", Topic::Pages),
    ("palette", Topic::Palette),
    ("pointer", Topic::Pointer),
    ("repos", Topic::Repos),
    ("settings", Topic::Settings),
    ("sidebar", Topic::Sidebar),
    ("tabs", Topic::Tabs),
];

/// How a [`PanelFact`] tests a topic's lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineTest {
    /// Some line holds this text.
    Has(String),
    /// No line holds it.
    Lacks(String),
    /// Some line is exactly this.
    Is(String),
    /// This many lines hold the text.
    Count(usize, String),
}

/// `expect TOPIC has|lacks|is|count ..`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelFact {
    /// What is read.
    pub topic: Topic,
    /// What must hold of it.
    pub test: LineTest,
}

/// The fact `what` and `tail` state, when `what` is a topic.
pub(super) fn parse(what: &str, tail: &str) -> Option<anyhow::Result<PanelFact>> {
    let topic = TOPICS.iter().find(|(word, _)| *word == what)?.1;
    Some(parse_test(tail).map(|test| PanelFact { topic, test }))
}

fn parse_test(tail: &str) -> anyhow::Result<LineTest> {
    let (verb, rest) = tail.split_once(' ').unwrap_or((tail, ""));
    let text = super::words::unescaped(rest.trim());
    Ok(match verb {
        "has" if !text.is_empty() => LineTest::Has(text),
        "lacks" if !text.is_empty() => LineTest::Lacks(text),
        "is" if !text.is_empty() => LineTest::Is(text),
        "count" => {
            let (count, text) = text.split_once(' ').unwrap_or((&text, ""));
            let count = count
                .parse()
                .map_err(|_| anyhow::anyhow!("`count` takes a number and the text to count"))?;
            LineTest::Count(count, text.to_owned())
        }
        _ => bail!("a topic is tested with `has TEXT`, `lacks TEXT`, `is LINE` or `count N TEXT`"),
    })
}

impl PanelFact {
    /// Whether the fact reads the effects of the step before, which only
    /// the runner that ran them has.
    pub fn reads_effects(&self) -> bool {
        self.topic == Topic::Effects
    }

    /// Whether the fact holds for `app`, with `sent` the effects of the
    /// last step that was not a check, when the runner kept them. The error
    /// lists the lines found.
    pub fn check(&self, app: &App, sent: Option<&[Effect]>) -> anyhow::Result<()> {
        let found = match (self.topic, sent) {
            (Topic::Effects, None) => {
                tracing::warn!("effects are not seen by this runner; the check is skipped");
                return Ok(());
            }
            (Topic::Effects, Some(sent)) => {
                let mut seen = Vec::new();
                for effect in sent {
                    lines::effect(effect, &mut seen);
                }
                seen
            }
            (topic, _) => lines::lines(app, topic),
        };
        let holding = |text: &str| found.iter().filter(|line| line.contains(text)).count();
        let shown = || {
            if found.is_empty() {
                " none".to_owned()
            } else {
                found.iter().fold(String::new(), |mut out, line| {
                    let _ = write!(out, "\n  {line}");
                    out
                })
            }
        };
        let name = TOPICS
            .iter()
            .find(|(_, topic)| *topic == self.topic)
            .map_or("?", |(word, _)| word);
        match &self.test {
            LineTest::Has(text) => {
                ensure!(
                    holding(text) > 0,
                    "no {name} line has `{text}`; the lines:{}",
                    shown()
                );
            }
            LineTest::Lacks(text) => {
                ensure!(
                    holding(text) == 0,
                    "a {name} line has `{text}`; the lines:{}",
                    shown()
                );
            }
            LineTest::Is(text) => ensure!(
                found.iter().any(|line| line == text),
                "no {name} line is `{text}`; the lines:{}",
                shown()
            ),
            LineTest::Count(count, text) => {
                let have = holding(text);
                ensure!(
                    have == *count,
                    "{have} {name} lines have `{text}`, expected {count}; the lines:{}",
                    shown()
                );
            }
        }
        Ok(())
    }
}

/// Every line of every topic but the effects, each as the `expect` that
/// holds it.
pub(super) fn dump(app: &App, out: &mut String) {
    for (word, topic) in TOPICS {
        if topic == Topic::Effects {
            continue;
        }
        for line in lines::lines(app, topic) {
            let _ = writeln!(out, "{word} has {}", super::words::escaped(&line));
        }
    }
}
