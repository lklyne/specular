//! The `keys` key: every binding with the command it runs.

use serde_json::{Value, json};

use crate::chrome::RESET_KEYS;
use crate::commands::{Run, naming};
use crate::{Action, BINDINGS, Binding, Context, KeyRow, Keymap, SHELL_KEYS};

const fn context_name(context: Context) -> &'static str {
    match context {
        Context::Always => "always",
        Context::Canvas => "canvas",
        Context::CanvasOrEditing => "canvas-or-editing",
        Context::Editing => "editing",
        Context::EnteredPage => "entered-page",
        Context::PageTarget => "page-target",
        Context::FrameTarget => "frame-target",
    }
}

/// The registry command a binding's action is, with its arguments. `None`
/// for an action no command says.
pub(super) fn command_of(action: &Action) -> Option<(&'static str, Value)> {
    if let Some(spec) = naming(&Run::Action(action.clone())) {
        return Some((spec.name, Value::Null));
    }
    match action {
        Action::SetTool(tool) => Some(("tool.set", json!({ "tool": tool.name() }))),
        Action::Nudge { dx, dy } => Some(("edit.nudge", json!({ "dx": dx, "dy": dy }))),
        Action::Canvas(crate::CanvasAction::Switch(id)) => {
            Some(("canvas.switch", json!({ "canvas": id.as_str() })))
        }
        _ => None,
    }
}

fn binding(row: &Binding) -> Value {
    let (command, args) = command_of(&row.action).unzip();
    json!({
        "keys": row.chord,
        "context": context_name(row.context),
        "command": command,
        "args": args.unwrap_or(Value::Null),
    })
}

/// A row of `keys.json`: `command` is `null` where it takes a key away.
fn person(row: &KeyRow) -> Value {
    let (command, args) = row.command.clone().unzip();
    json!({
        "keys": row.chord,
        "context": row.section.name(),
        "command": command,
        "args": args.unwrap_or(Value::Null),
        "from": "keys.json",
    })
}

/// The person's keys from `keys.json`, which come first, then every
/// binding in the table's order, then the window's own keys, then the
/// reset key, which nothing can rebind.
pub(super) fn bindings(keymap: &Keymap) -> Value {
    let mine = keymap.rows().iter().map(person);
    let table = BINDINGS.iter().map(binding);
    let shell = SHELL_KEYS.iter().map(|(chord, command)| {
        json!({
            "keys": chord,
            "context": "always",
            "command": naming(&Run::Shell(*command)).map(|spec| spec.name),
            "args": Value::Null,
        })
    });
    let reset = json!({
        "keys": RESET_KEYS,
        "context": "always",
        "command": "ui.reset",
        "args": Value::Null,
        "fixed": true,
    });
    Value::Array(mine.chain(table).chain(shell).chain([reset]).collect())
}
