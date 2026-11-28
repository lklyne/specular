//! The dock for mods: the buttons of a mod's own commands that ask for one
//! (`"dock": true` in `mod.json`), while its tool is in hand or its items
//! are selected. Pressing one runs the command with the selected item ids.

use serde_json::{Value, json};
use specular_doc::{Entity, Kind};

use super::super::build::{button, groups, noun};
use super::super::{Control, ControlId, ControlsModel, Face, Icon};
use super::actions::Actions;
use crate::App;
use crate::mods::{ModToolId, call_action, running_mods, word};

/// The dock buttons of the running mod `name`, each run with `args`.
fn buttons(app: &App, name: &str, args: &Value) -> Vec<Control> {
    let Some(installed) = running_mods(app).into_iter().find(|each| each.name == name) else {
        return Vec::new();
    };
    (installed.manifest.commands.iter())
        .filter(|spec| spec.dock)
        .filter_map(|spec| {
            let word = word(installed, spec);
            let action = call_action(app, &word, args)?;
            let icon = Icon::named_or(spec.icon.as_deref(), Icon::Code);
            let face = Face {
                text: spec.icon.is_none().then(|| spec.title.clone().into()),
                ..Face::icon(icon)
            };
            let id = ControlId::new("mod").child(&word);
            Some(button(id, spec.title.clone(), face, action))
        })
        .collect()
}

/// Whether the mod tool `id`'s mod has dock buttons, which then take the
/// place of any selection's controls.
pub(super) fn tool_has_options(app: &App, id: ModToolId) -> bool {
    tool_controls(app, id).is_some()
}

/// The dock while the mod tool `id` is in hand.
pub(super) fn tool_controls(app: &App, id: ModToolId) -> Option<ControlsModel> {
    let (installed, _) = app.mods.catalog.tool(id)?;
    let controls = buttons(app, &installed.name, &json!({ "ids": [] }));
    (!controls.is_empty()).then_some(ControlsModel { controls })
}

/// The dock for selected mod items: their mod's buttons when they share
/// one, then arranging and annotating.
pub(super) fn controls(app: &App, entities: &[&Entity]) -> ControlsModel {
    let name = |entity: &&Entity| {
        let Kind::Mod(item) = &entity.kind else {
            return None;
        };
        Some(item.name.clone())
    };
    let first = entities.first().and_then(name);
    let shared = first.filter(|first| entities.iter().all(|e| name(e).as_ref() == Some(first)));
    let ids: Vec<&str> = entities.iter().map(|entity| entity.id.as_str()).collect();
    let own = shared.map_or_else(Vec::new, |name| buttons(app, &name, &json!({ "ids": ids })));
    let noun = noun(entities.len(), "mod item", "mod items");
    ControlsModel {
        controls: groups(vec![own, Actions::all(&noun, entities.len()).controls()]),
    }
}
