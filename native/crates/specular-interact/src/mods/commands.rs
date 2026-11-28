//! Commands a mod adds and handles itself (`commands` in `mod.json`): listed
//! in the command palette beside the app's own, bound to their keys on the
//! canvas, and shown as dock buttons while the mod's tool is in hand or its
//! items are selected. Running one publishes the `call` key,
//! `{command, number, args}`, which the mod reads and acts on once per
//! `number`.

use serde::Serialize;
use serde_json::{Value, json};

use super::{InstalledMod, ModAction, ModCommand};
use crate::{Action, App, Chord};

/// The last mod command run, as the `call` key holds it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ModCall {
    /// `<mod>/<command>`.
    pub(crate) command: String,
    /// Counts up with each call, so a mod tells a repeat from the last one.
    pub(crate) number: u64,
    /// What it was run with.
    pub(crate) args: Value,
}

/// The mods running now: those the layout fills a region or an overlay
/// with, then those loaded out of sight ([`ModCatalog::background`]).
///
/// [`ModCatalog::background`]: super::ModCatalog::background
pub fn running(app: &App) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let filled = (app.chrome.layout.nodes()).filter_map(|(_, node)| node.fill.clone());
    let background = (app.mods.catalog.background()).map(|installed| installed.name.clone());
    for name in filled.chain(background) {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// The installed mods that are running, in [`running`]'s order.
pub(crate) fn running_mods(app: &App) -> Vec<&InstalledMod> {
    (running(app).iter())
        .filter_map(|name| app.mods.catalog.get(name))
        .collect()
}

/// `<mod>/<command>`.
pub(crate) fn word(installed: &InstalledMod, spec: &ModCommand) -> String {
    format!("{}/{}", installed.name, spec.name)
}

/// The action that runs the mod command `word` with `args`, when a running
/// mod has it.
pub(crate) fn call_action(app: &App, word: &str, args: &Value) -> Option<Action> {
    let (installed, _) = app.mods.catalog.command_named(word)?;
    running(app).contains(&installed.name).then(|| {
        Action::Mod(Box::new(ModAction::Call {
            command: word.to_owned(),
            args: args.clone(),
        }))
    })
}

/// Publishes a call of `command` for its mod to act on.
pub(crate) fn call(app: &mut App, command: String, args: Value) {
    let number = app.mods.call.as_ref().map_or(1, |last| last.number + 1);
    app.mods.call = Some(ModCall {
        command,
        number,
        args,
    });
}

/// The running mod command whose key is `chord`.
pub(crate) fn bound_to(app: &App, chord: Chord) -> Option<Action> {
    running_mods(app).into_iter().find_map(|installed| {
        let spec = (installed.manifest.commands.iter())
            .find(|spec| spec.key.as_deref().and_then(Chord::parse) == Some(chord))?;
        call_action(app, &word(installed, spec), &Value::Null)
    })
}

/// The `call` key.
pub(crate) fn call_json(app: &App) -> Value {
    app.mods
        .call
        .as_ref()
        .map_or(Value::Null, |call| json!(call))
}
