//! `keys.json` (ADR 0059): the person's own keys over the binding table.
//!
//! ```json
//! {
//!   "global":  { "cmd+shift+l": "layout.switch" },
//!   "canvas":  { "s": { "command": "tool.set", "args": { "tool": "shape" } }, "r": null },
//!   "editing": { },
//!   "page":    { },
//!   "layout:studio": { "f": "ui.toggle:{\"id\":\"sidebar\"}" }
//! }
//! ```
//!
//! A section says where its keys fire: `global` everywhere, `canvas` while
//! keys go to the canvas and no text is edited, `editing` while text is,
//! `page` while a page is entered, and `layout:<name>` as `canvas` does
//! while that layout is in force. A key names a command of the registry,
//! with its arguments after a `:` as JSON or as `args`; `null` takes the
//! table's key away there, so it reaches what is under it. The person's
//! keys come before the table's. The reset key comes before both.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::commands::{Run, RunError, command};
use crate::{Action, App, Binding, Chord, Context};

/// Where a row of `keys.json` fires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySection {
    /// Everywhere, an entered page included.
    Global,
    /// While keys go to the canvas and no text is edited.
    Canvas,
    /// While text is edited.
    Editing,
    /// While a page is entered.
    Page,
    /// As `Canvas`, while the layout named is in force.
    Layout(String),
}

impl KeySection {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "global" => Self::Global,
            "canvas" => Self::Canvas,
            "editing" => Self::Editing,
            "page" => Self::Page,
            _ => Self::Layout(name.strip_prefix("layout:")?.to_owned()),
        })
    }

    /// The section's name as `keys.json` writes it.
    pub fn name(&self) -> String {
        match self {
            Self::Global => "global".to_owned(),
            Self::Canvas => "canvas".to_owned(),
            Self::Editing => "editing".to_owned(),
            Self::Page => "page".to_owned(),
            Self::Layout(name) => format!("layout:{name}"),
        }
    }

    /// The table's context the section stands for.
    pub const fn context(&self) -> Context {
        match self {
            Self::Global => Context::Always,
            Self::Canvas | Self::Layout(_) => Context::Canvas,
            Self::Editing => Context::Editing,
            Self::Page => Context::EnteredPage,
        }
    }

    pub(crate) fn holds(&self, app: &App) -> bool {
        let layout = match self {
            Self::Layout(name) => app.layout_name() == name,
            _ => true,
        };
        layout && self.context().holds(app)
    }
}

/// One row of `keys.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyRow {
    /// The key and modifiers.
    pub chord: Chord,
    /// Where it fires.
    pub section: KeySection,
    /// The command and its arguments, or `None` where the key is taken
    /// away.
    pub command: Option<(String, Value)>,
    /// What the command builds, made once as the file is read.
    pub(crate) run: Option<Run>,
}

/// The person's keys, in the file's order with `layout:` sections first.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Keymap {
    rows: Vec<KeyRow>,
}

/// The command a value names, with its arguments.
fn command_of(value: &Value) -> Result<Option<(String, Value)>, String> {
    match value {
        Value::Null => Ok(None),
        Value::String(text) => {
            let (name, args) = match text.split_once(':') {
                Some((name, args)) => {
                    (name, serde_json::from_str(args).map_err(|e| e.to_string())?)
                }
                None => (text.as_str(), Value::Null),
            };
            Ok(Some((name.to_owned(), args)))
        }
        Value::Object(fields) => {
            let name = (fields.get("command").and_then(Value::as_str))
                .ok_or("an object names its `command`")?;
            let args = fields.get("args").cloned().unwrap_or(Value::Null);
            Ok(Some((name.to_owned(), args)))
        }
        _ => Err("a key names a command, `{command, args}` or null".to_owned()),
    }
}

fn build(name: &str, args: &Value) -> Result<Run, String> {
    let spec = command(name).ok_or_else(|| {
        RunError::Unknown {
            name: name.to_owned(),
            closest: Vec::new(),
        }
        .to_string()
    })?;
    (spec.build)(args).map_err(|error| format!("{name}: {error}"))
}

impl Keymap {
    /// Reads `keys.json`. An error names the section and key it is in.
    pub fn parse(text: &str) -> Result<Self, String> {
        let file: BTreeMap<String, BTreeMap<String, Value>> =
            serde_json::from_str(text).map_err(|error| error.to_string())?;
        let mut rows = Vec::new();
        for (section_name, keys) in &file {
            let section = KeySection::parse(section_name).ok_or_else(|| {
                format!("`{section_name}` is not global, canvas, editing, page or layout:<name>")
            })?;
            for (keys_text, value) in keys {
                let at = |reason: String| format!("{section_name} \"{keys_text}\": {reason}");
                let chord = Chord::parse(keys_text).ok_or_else(|| at("not a key".to_owned()))?;
                let command = command_of(value).map_err(at)?;
                let run = match &command {
                    Some((name, args)) => Some(build(name, args).map_err(at)?),
                    None => None,
                };
                rows.push(KeyRow {
                    chord,
                    section: section.clone(),
                    command,
                    run,
                });
            }
        }
        rows.sort_by_key(|row| !matches!(row.section, KeySection::Layout(_)));
        Ok(Self { rows })
    }

    /// Every row, `layout:` sections first.
    pub fn rows(&self) -> &[KeyRow] {
        &self.rows
    }

    /// The key a menu shows for `action`: the person's for it, else the
    /// table's `table` unless they took it away.
    pub(crate) fn chord_for(&self, action: &Action, table: Option<&Binding>) -> Option<Chord> {
        let general = |row: &&KeyRow| !matches!(row.section, KeySection::Layout(_));
        let mine = (self.rows.iter().filter(general))
            .find(|row| matches!(&row.run, Some(Run::Action(own)) if own == action));
        if let Some(row) = mine {
            return Some(row.chord);
        }
        let table = table?;
        let taken = (self.rows.iter().filter(general))
            .any(|row| row.chord == table.chord && row.section.context() == table.context);
        (!taken).then_some(table.chord)
    }

    /// The row `chord` fires in `app` now.
    pub(crate) fn row_for(&self, app: &App, chord: Chord) -> Option<&KeyRow> {
        (self.rows.iter()).find(|row| row.chord == chord && row.section.holds(app))
    }
}
