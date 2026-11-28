//! The Mods page of the settings: each installed mod, what it adds, and
//! its switch.

use crate::{Action, App, SettingAction};

/// One installed mod, as the Mods page lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct ModRow {
    /// Its folder name, the name the switch's control goes by.
    pub name: String,
    /// What it is called.
    pub title: String,
    /// What it adds, in a line.
    pub detail: String,
    /// Whether it is turned on.
    pub on: bool,
    /// Turns it the other way.
    pub action: Action,
}

/// Every installed mod, in name order.
pub(crate) fn mods_pane(app: &App) -> Vec<ModRow> {
    let layout = app.chrome.written_layout();
    let placed: Vec<String> = (layout.mods().into_iter())
        .map(|(_, name, _)| name.to_owned())
        .collect();
    (app.mods.installed.mods.iter())
        .map(|installed| {
            let on = app.settings.mod_on(installed);
            let manifest = &installed.manifest;
            let mut adds = Vec::new();
            if placed.contains(&installed.name) {
                adds.push("in this layout".to_owned());
            }
            if manifest.items.is_some() {
                adds.push("canvas items".to_owned());
            }
            adds.extend(count(manifest.tools.len(), "tool"));
            adds.extend(count(manifest.commands.len(), "command"));
            let detail = if adds.is_empty() {
                installed.name.clone()
            } else {
                format!("{}: {}", installed.name, adds.join(", "))
            };
            ModRow {
                name: installed.name.clone(),
                title: manifest.title.clone(),
                detail,
                on,
                action: Action::Setting(SettingAction::Mod(installed.name.clone(), !on)),
            }
        })
        .collect()
}

fn count(n: usize, what: &str) -> Option<String> {
    match n {
        0 => None,
        1 => Some(format!("1 {what}")),
        n => Some(format!("{n} {what}s")),
    }
}
