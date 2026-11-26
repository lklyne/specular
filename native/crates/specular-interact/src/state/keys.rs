//! The `keys` key: every binding with the command it runs.

use serde_json::{Value, json};

use crate::chrome::RESET_KEYS;
use crate::commands::{Run, naming};
use crate::{Action, BINDINGS, Binding, Context, SHELL_KEYS};

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
fn command_of(action: &Action) -> Option<(&'static str, Value)> {
    if let Some(spec) = naming(&Run::Action(action.clone())) {
        return Some((spec.name, Value::Null));
    }
    match action {
        Action::SetTool(tool) => Some(("tool.set", json!({ "tool": tool.name() }))),
        Action::Nudge { dx, dy } => Some(("edit.nudge", json!({ "dx": dx, "dy": dy }))),
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

/// Every binding in the table's order, then the window's own keys, then
/// the reset key, which nothing can rebind.
pub(super) fn bindings() -> Value {
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
    Value::Array(table.chain(shell).chain([reset]).collect())
}
