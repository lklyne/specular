//! The command registry: everything a person, a script or an agent can run
//! by name, in one table.
//!
//! A command is a dotted lower-case name (`edit.undo`, `tool.set`), a line
//! of help for its arguments, a sentence on what it does, whether it can run
//! now, and how its JSON arguments become a [`Run`]. The menus, the palette,
//! the context menu, the scenario scripts, the CLI and the HTTP API's
//! `POST /run` read this table, so a name and a label are written once.
//!
//! What [`CommandSpec::enabled`] says is whether the command has something
//! to act on. Whether its key reaches it where the keys are (a page entered,
//! a field being typed in) is the menus' business, not the registry's: a
//! command run by name is not a key press.

mod args;
mod canvas;
mod edit;
mod mods;
mod page;
mod selection;
mod ui;
mod view;

use std::sync::LazyLock;

use serde_json::Value;

use crate::{Action, ApiCall, ApiRun, App, Event, ShellCommand, UnknownControl, control_named};

/// One command of the registry.
#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    /// Dotted and lower case: the area, then the verb (`view.zoom-in`).
    pub name: &'static str,
    /// The short title a menu item or a palette row shows. Empty for a
    /// command that is in neither.
    pub label: &'static str,
    /// One line of help for the JSON arguments, such as `{tool:
    /// select|page|..}`. Empty when it takes none.
    pub args: &'static str,
    /// One sentence on what it does.
    pub about: &'static str,
    /// Whether it has something to act on in `app` now.
    pub enabled: fn(&App) -> bool,
    /// What it does with these arguments. A command with no arguments
    /// ignores what it is given.
    pub build: fn(&Value) -> Result<Run, RunError>,
}

impl CommandSpec {
    /// The command built with no arguments, as a menu item or a palette row
    /// runs it. `None` for a command that needs arguments.
    pub(crate) fn plain(&self) -> Option<Run> {
        (self.build)(&Value::Null).ok()
    }
}

/// What running a command does.
#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    /// An action through [`update`](crate::update).
    Action(Action),
    /// Something the window does itself, outside `update`.
    Shell(ShellCommand),
    /// A change the HTTP API's way runs, which no [`Action`] says.
    Api(ApiRun),
}

impl Run {
    /// The event that does it, with an API change carrying ticket 0, or the
    /// window's own command, which is no event.
    pub fn into_event(self) -> Result<Event, ShellCommand> {
        match self {
            Self::Action(action) => Ok(Event::Action(action)),
            Self::Api(run) => Ok(Event::Api(ApiCall {
                ticket: 0,
                canvas: None,
                thread: None,
                run,
            })),
            Self::Shell(command) => Err(command),
        }
    }
}

/// Why a command did not run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RunError {
    /// No command has the name.
    #[error("no command `{name}`{}", suggest(closest))]
    Unknown {
        /// The name asked for.
        name: String,
        /// The names nearest to it, nearest first.
        closest: Vec<&'static str>,
    },
    /// The command has nothing to act on now.
    #[error("`{0}` cannot run now")]
    Disabled(&'static str),
    /// The arguments are not the ones it takes.
    #[error("{0}")]
    Args(String),
    /// `control.press` named a control there is not.
    #[error(transparent)]
    Control(#[from] UnknownControl),
}

fn suggest(closest: &[&str]) -> String {
    if closest.is_empty() {
        String::new()
    } else {
        format!("; did you mean {}?", closest.join(", "))
    }
}

static ALL: LazyLock<Vec<CommandSpec>> = LazyLock::new(|| {
    [
        canvas::APP,
        canvas::CANVAS,
        edit::EDIT,
        edit::ITEM,
        edit::TEXT,
        selection::ARRANGE,
        selection::COMMENT,
        page::PAGE,
        page::TAB,
        view::TOOL,
        view::VIEW,
        ui::UI,
        ui::NOTICE,
        mods::MODS,
        canvas::SHARE,
        canvas::CONTROL,
    ]
    .concat()
});

/// Every command, by area.
pub fn commands() -> &'static [CommandSpec] {
    &ALL
}

/// The command named `name`.
pub fn command(name: &str) -> Option<&'static CommandSpec> {
    commands().iter().find(|spec| spec.name == name)
}

/// The command whose plain build is `run`: where a menu item or a key's
/// action finds its label.
pub(crate) fn naming(run: &Run) -> Option<&'static CommandSpec> {
    (commands().iter()).find(|spec| spec.plain().as_ref() == Some(run))
}

/// What running `name` with `args` on `app` does, refused for a name no
/// command has (with the nearest names), for arguments it does not take,
/// for a command with nothing to act on, and for a control that is not
/// there.
pub fn run_named(app: &App, name: &str, args: &Value) -> Result<Run, RunError> {
    if let Some(action) = crate::mods::call_action(app, name, args) {
        return Ok(Run::Action(action));
    }
    let spec = command(name).ok_or_else(|| RunError::Unknown {
        name: name.to_owned(),
        closest: closest(name),
    })?;
    let run = (spec.build)(args).map_err(|error| match error {
        RunError::Args(reason) => RunError::Args(format!("`{name}` takes {}: {reason}", spec.args)),
        other => other,
    })?;
    if !(spec.enabled)(app) {
        return Err(RunError::Disabled(spec.name));
    }
    if let Run::Action(Action::PressControl(id)) = &run {
        control_named(app, id.as_str())?;
    }
    Ok(run)
}

/// Up to three names near `name`: ones it is part of, then the fewest
/// edits away.
fn closest(name: &str) -> Vec<&'static str> {
    let mut near: Vec<(usize, &'static str)> = (commands().iter())
        .filter_map(|spec| {
            let distance = if spec.name.contains(name) || name.contains(spec.name) {
                0
            } else {
                edits(name, spec.name)
            };
            (distance <= spec.name.len().max(name.len()) / 3).then_some((distance, spec.name))
        })
        .collect();
    near.sort_unstable();
    near.into_iter().take(3).map(|(_, name)| name).collect()
}

/// The Levenshtein distance between `a` and `b`, in characters.
fn edits(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(ca != cb));
            diagonal = above;
        }
    }
    row[b.len()]
}
