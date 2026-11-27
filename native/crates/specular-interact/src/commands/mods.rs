//! Mod items and tools (ADR 0057): what a mod runs to draw its items,
//! change their props and preview a stroke, and the canvas's install ask.

use glam::DVec2;
use serde_json::Value;
use specular_doc::{JsonMap, ModDraw};

use super::args::{act, always, entity, idle, text};
use super::{CommandSpec, Run, RunError};
use crate::mods::{ModAction, NewModItem};
use crate::{Action, App};

fn wrong(reason: &str) -> RunError {
    RunError::Args(reason.to_owned())
}

/// The object under `key`, when there is one.
fn object(args: &Value, key: &str) -> Result<Option<JsonMap>, RunError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(map)) => Ok(Some(map.clone())),
        Some(_) => Err(wrong(&format!("`{key}` is an object"))),
    }
}

/// The display list under `draws`.
fn draws(args: &Value) -> Result<Vec<ModDraw>, RunError> {
    (args.get("draws").and_then(ModDraw::list))
        .ok_or_else(|| wrong("`draws` is a list of {type: rect|ellipse|path|text|image, ..}"))
}

/// `item.add`'s mod arguments: `mod`, `props`, and `width` and `height`
/// together or neither.
pub(super) fn new_mod_item(args: &Value) -> Result<NewModItem, RunError> {
    let size = match (
        args.get("width").and_then(Value::as_f64),
        args.get("height").and_then(Value::as_f64),
    ) {
        (Some(width), Some(height)) if width > 0.0 && height > 0.0 => {
            Some(DVec2::new(width, height))
        }
        (None, None) => None,
        _ => {
            return Err(wrong(
                "`width` and `height` are numbers above 0, both or neither",
            ));
        }
    };
    Ok(NewModItem {
        name: text(args, "mod")?,
        props: object(args, "props")?.unwrap_or_default(),
        size,
    })
}

/// Whether the active canvas needs a mod that is not installed.
fn missing(app: &App) -> bool {
    !crate::mods::missing(app).is_empty()
}

fn run(action: ModAction) -> Result<Run, RunError> {
    act(Action::Mod(Box::new(action)))
}

/// What mods run for their items and tools.
pub(super) const MODS: &[CommandSpec] = &[
    CommandSpec {
        name: "item.set",
        label: "",
        args: "{id: mod item id, props: object}",
        about: "Set keys of a mod item's props, one undo step; a null takes a key away.",
        enabled: idle,
        build: |args| {
            let props = object(args, "props")?.ok_or_else(|| wrong("`props` is required"))?;
            run(ModAction::SetProps {
                id: entity(args, "id")?,
                props,
            })
        },
    },
    CommandSpec {
        name: "item.draw",
        label: "",
        args: "{id: mod item id, draws: [{type: rect|ellipse|path|text|image, ..}]}",
        about: "Give a mod item the display list it shows, in item space. Not an undo step.",
        enabled: always,
        build: |args| {
            run(ModAction::Draw {
                id: entity(args, "id")?,
                draws: draws(args)?,
            })
        },
    },
    CommandSpec {
        name: "tool.preview",
        label: "",
        args: "{draws: [{type: rect|ellipse|path|text|image, ..}]}",
        about: "Show what the armed mod tool would make, in canvas units; an empty list clears it.",
        enabled: always,
        build: |args| run(ModAction::Preview(draws(args)?)),
    },
    CommandSpec {
        name: "mods.install",
        label: "Install mods",
        args: "",
        about: "Install the mods this canvas uses and the app does not have, from the space folder.",
        enabled: missing,
        build: |_| run(ModAction::Install),
    },
];
