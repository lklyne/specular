//! The "Install mods" ask: a canvas whose items need a mod that is not
//! installed says so in the `notices` key, naming each mod and where it
//! comes from, and offers `mods.install`.
//!
//! Opening a canvas never installs or runs a mod. Installing copies the mod
//! from the space folder (where the canvas's `specular.mods` says it is)
//! into the app home, which is running code with the person's access, and
//! the notice says so. Until then the items show their snapshot or a
//! placeholder, and their props are kept untouched. Declining is kept for
//! the canvas for the rest of the run.

use specular_doc::{Kind, ModNeed};

use crate::{App, AppNotice, Effect, NoticeAction};

/// The notice's id, which `notice.dismiss` takes to decline.
pub(crate) const NOTICE: &str = "mods.install";

/// The mods the active canvas needs that are not installed: those its list
/// names, then those its items name that the list does not, each from
/// `mods/<name>` in the space folder.
pub(crate) fn missing(app: &App) -> Vec<ModNeed> {
    let mut needs = app.document.mod_needs();
    for entity in app.document.entities() {
        if let Kind::Mod(item) = &entity.kind
            && !needs.iter().any(|need| need.name == item.name)
        {
            needs.push(ModNeed {
                name: item.name.clone(),
                api: String::new(),
                from: format!("mods/{}", item.name),
            });
        }
    }
    let catalog = &app.mods.catalog;
    needs.retain(|need| catalog.get(&need.name).is_none());
    needs
}

/// The ask for the active canvas, unless nothing is missing or it was
/// declined.
pub(crate) fn install_notice(app: &App) -> Option<AppNotice> {
    let needs = missing(app);
    if needs.is_empty() || app.mods.declined.contains(app.space.active().id.as_str()) {
        return None;
    }
    let names: Vec<String> = (needs.iter())
        .map(|need| format!("{} (from {})", need.name, need.from))
        .collect();
    Some(AppNotice {
        id: NOTICE.to_owned(),
        level: "warning",
        text: format!(
            "This canvas uses mods that are not installed: {}. Installing them runs their code \
             with your access.",
            names.join(", ")
        ),
        action: Some(NoticeAction {
            label: "Install mods",
            run: "mods.install",
            args: serde_json::Value::Null,
        }),
        dismiss: true,
    })
}

/// Installs what the active canvas is missing.
pub(crate) fn install_missing(app: &App, effects: &mut Vec<Effect>) {
    let needs = missing(app);
    if !needs.is_empty() {
        effects.push(Effect::InstallMods(needs));
    }
}

/// Declines the ask for the active canvas. Returns whether `id` was the ask.
pub(crate) fn decline(app: &mut App, id: &str) -> bool {
    if id != NOTICE {
        return false;
    }
    let canvas = app.space.active().id.as_str().to_owned();
    app.mods.declined.insert(canvas)
}
