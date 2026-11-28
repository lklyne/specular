//! [`ModAction`]: what a mod, or the person, asks of mod items and tools.

use specular_doc::{EntityId, JsonMap, ModDraw};

use crate::update::{document_step, set_tool, verb};
use crate::{App, Effect, Tool, images};

/// A mod item or tool action.
#[derive(Debug, Clone, PartialEq)]
pub enum ModAction {
    /// Arm a mod's tool by its word, `<mod>/<tool>`. A word no installed
    /// mod has does nothing.
    SetTool(String),
    /// Set keys of a mod item's props as one undo step; a `null` takes its
    /// key away.
    SetProps {
        /// The mod item.
        id: EntityId,
        /// The keys to set.
        props: JsonMap,
    },
    /// Change the values and outline of mod shapes, one undo step.
    SetShapes(super::ShapeChange),
    /// A mod item's mod sent what the item shows. Not an undo step.
    Draw {
        /// The mod item.
        id: EntityId,
        /// Its display list, in item space.
        draws: Vec<ModDraw>,
    },
    /// The armed mod tool's preview, in canvas units. Not an undo step.
    Preview(Vec<ModDraw>),
    /// Install the mods the active canvas needs and the app lacks.
    Install,
    /// Run a mod's own command, `<mod>/<command>`: published as the `call`
    /// key for the mod to act on.
    Call {
        /// `<mod>/<command>`.
        command: String,
        /// What it is run with.
        args: serde_json::Value,
    },
}

/// Runs `action`.
pub(crate) fn run(app: &mut App, action: ModAction, effects: &mut Vec<Effect>) {
    match action {
        ModAction::SetTool(word) => {
            if let Some(id) = app.mods.catalog.tool_named(&word) {
                set_tool(app, Tool::Mod(id), effects);
            }
        }
        ModAction::SetProps { id, props } => verb(app, effects, |app, fx| {
            if let Some(command) = super::set_props(app, &id, &props) {
                document_step(app, command, fx);
            }
        }),
        ModAction::SetShapes(change) => verb(app, effects, |app, fx| {
            if let Some(command) = super::set_shapes(app, &change) {
                document_step(app, command, fx);
            }
        }),
        ModAction::Draw { id, draws } => {
            if super::on_draw(app, id, draws) {
                effects.push(Effect::Save);
            }
            images::request_new(app, effects);
        }
        ModAction::Preview(draws) => app.mods.preview = draws,
        ModAction::Install => super::install_missing(app, effects),
        ModAction::Call { command, args } => super::commands::call(app, command, args),
    }
}
