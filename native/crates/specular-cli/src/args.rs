//! The command line split into a verb, its positionals and its flags.
//!
//! Flags may sit anywhere. A flag in [`VALUE_FLAGS`] takes the next word as
//! its value; any other `--name` or `-x` is a switch. `--` ends the flags.
//! Unknown verbs pass through to agent-browser, so its own flags must parse
//! here too: they are listed with the rest.

use std::collections::{HashMap, HashSet};

/// The flags that take a value.
const VALUE_FLAGS: &[&str] = &[
    "--page",
    "-f",
    // The canvas a verb targets. Only the tab-scoped routes take it; the
    // rest answer 400 rather than write to the canvas the user is on.
    "--tab",
    "--preset",
    "--presets",
    "--at",
    "--width",
    "--height",
    "--size",
    "--cols",
    "--anchor",
    "--label",
    "--color",
    "--layout",
    "--gap",
    "--status",
    "--url",
    "--page-id",
    "--ids",
    "--reason",
    "--text",
    "--kind",
    // agent-browser's.
    "-s",
    "-d",
    "-p",
    "--selector",
    "--format",
    "--depth",
    "--wait",
    "--attr",
    "--timeout",
    "--load",
    "--viewport",
    "--device",
    "--baseline",
    "--screenshot-format",
    "--screenshot-quality",
    "--screenshot-dir",
    "--max-output",
    "--download-path",
    "--color-scheme",
    "--idle-timeout",
];

/// A parsed command line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Args {
    /// The first positional word.
    pub(crate) verb: String,
    /// The positionals after the verb.
    pub(crate) positional: Vec<String>,
    /// The value flags, by name without dashes.
    pub(crate) flags: HashMap<String, String>,
    /// The switches, by name without dashes.
    pub(crate) switches: HashSet<String>,
    /// Every word but the verb, as given, for a passthrough to
    /// agent-browser.
    pub(crate) rest: Vec<String>,
}

impl Args {
    /// The value of flag `name`.
    pub(crate) fn flag(&self, name: &str) -> Option<&str> {
        self.flags.get(name).map(String::as_str)
    }

    /// Whether switch `name` was given.
    pub(crate) fn switch(&self, name: &str) -> bool {
        self.switches.contains(name)
    }

    /// The positionals from `from` on, joined by spaces.
    pub(crate) fn words(&self, from: usize) -> String {
        self.positional.get(from..).unwrap_or_default().join(" ")
    }

    /// The positional at `index`.
    pub(crate) fn nth(&self, index: usize) -> Option<&str> {
        self.positional.get(index).map(String::as_str)
    }

    /// The page a browse verb drives: `--page` or `-f`.
    pub(crate) fn page(&self) -> Option<&str> {
        self.flag("page").or_else(|| self.flag("f"))
    }
}

fn is_flag(word: &str) -> bool {
    word.starts_with("--") || (word.starts_with('-') && word.chars().count() == 2)
}

/// Splits `argv` (without the program name).
pub(crate) fn parse(argv: &[String]) -> Args {
    let mut args = Args::default();
    let mut positional = Vec::new();
    let mut verb_at = None;
    let mut at = 0;
    while let Some(word) = argv.get(at) {
        if word == "--" {
            positional.extend(argv.get(at + 1..).unwrap_or_default().iter().cloned());
            if verb_at.is_none() && at + 1 < argv.len() {
                verb_at = Some(at + 1);
            }
            break;
        }
        if is_flag(word) {
            let name = word.trim_start_matches('-').to_owned();
            match argv.get(at + 1) {
                Some(value) if VALUE_FLAGS.contains(&word.as_str()) => {
                    args.flags.insert(name, value.clone());
                    at += 2;
                }
                _ => {
                    args.switches.insert(name);
                    at += 1;
                }
            }
            continue;
        }
        if verb_at.is_none() {
            verb_at = Some(at);
        }
        positional.push(word.clone());
        at += 1;
    }
    let mut positional = positional.into_iter();
    args.verb = positional.next().unwrap_or_default();
    args.positional = positional.collect();
    args.rest = (argv.iter().enumerate())
        .filter(|(index, _)| Some(*index) != verb_at)
        .map(|(_, word)| word.clone())
        .collect();
    args
}
