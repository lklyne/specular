//! Shipped mods someone edited in place. A start writes the shipped mods
//! over their folders, so a folder whose files differ from what the app
//! last wrote is moved to `<name>-edited` first, as the person's own copy,
//! and the `notices` key says where it went until it is dismissed. A copy
//! that would run with no layout placing it (tools, shapes, commands)
//! starts turned off, so the shipped mod and its copy do not both run.

use super::InstalledMod;
use crate::{App, AppNotice, Effect};

/// The notice's id, which `notice.dismiss` takes.
pub(crate) const NOTICE: &str = "mods.set-aside";

/// A shipped mod's edited folder, moved aside before the shipped one was
/// written back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetAside {
    /// The shipped mod's name.
    pub name: String,
    /// The folder name the edited copy now has.
    pub to: String,
}

/// The shell moved `moved` aside before listing what is installed.
pub(crate) fn on_set_aside(app: &mut App, moved: Vec<SetAside>) {
    app.mods.unsettled = moved.iter().map(|each| each.to.clone()).collect();
    app.mods.set_aside = moved;
}

/// Turns off each set-aside copy the catalog now lists that runs
/// unplaced, once: the person may turn it back on.
pub(crate) fn settle(app: &mut App, effects: &mut Vec<Effect>) {
    let before = app.settings.clone();
    let mut unsettled = std::mem::take(&mut app.mods.unsettled);
    unsettled.retain(|to| {
        let Some(installed) = app.mods.installed.get(to) else {
            return true;
        };
        if runs_unplaced(installed) {
            app.settings.mods_on.remove(to);
            app.settings.mods_off.insert(to.clone());
        }
        false
    });
    app.mods.unsettled = unsettled;
    if app.settings != before {
        effects.push(Effect::SaveSettings(app.settings.clone()));
    }
}

fn runs_unplaced(installed: &InstalledMod) -> bool {
    installed.manifest.runs_unplaced()
}

/// What the last start set aside, as a notice.
pub(crate) fn notice(app: &App) -> Option<AppNotice> {
    if app.mods.set_aside.is_empty() {
        return None;
    }
    let moved: Vec<String> = (app.mods.set_aside.iter())
        .map(|each| format!("`{}` to `{}`", each.name, each.to))
        .collect();
    let (off, placed): (Vec<&SetAside>, Vec<&SetAside>) = (app.mods.set_aside.iter())
        .partition(|each| app.mods.installed.get(&each.to).is_some_and(runs_unplaced));
    let mut text = vec![format!(
        "Your edits to shipped mods were kept as copies: {}.",
        moved.join(", ")
    )];
    if let Some((names, them)) = listed(&off) {
        text.push(format!("Turn on {names} in Settings > Mods to use {them}."));
    }
    if let Some((names, them)) = listed(&placed) {
        text.push(format!("Point a layout at {names} to use {them}."));
    }
    Some(AppNotice {
        id: NOTICE.to_owned(),
        level: "info",
        text: text.join(" "),
        action: None,
        dismiss: true,
        about: None,
    })
}

/// The copies' folder names, and the pronoun for them. `None` for none.
fn listed(copies: &[&SetAside]) -> Option<(String, &'static str)> {
    let names: Vec<String> = (copies.iter())
        .map(|each| format!("`{}`", each.to))
        .collect();
    match names.len() {
        0 => None,
        1 => Some((names.join(", "), "it")),
        _ => Some((names.join(", "), "them")),
    }
}

/// Takes the notice away. Returns whether `id` was it.
pub(crate) fn dismiss(app: &mut App, id: &str) -> bool {
    id == NOTICE && !std::mem::take(&mut app.mods.set_aside).is_empty()
}
