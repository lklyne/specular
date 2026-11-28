//! Shipped mods someone edited in place. A start writes the shipped mods
//! over their folders, so a folder whose files differ from what the app
//! last wrote is moved to `<name>-edited` first, as the person's own copy,
//! and the `notices` key says where it went until it is dismissed.

use crate::{App, AppNotice};

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

/// What the last start set aside, as a notice.
pub(crate) fn notice(app: &App) -> Option<AppNotice> {
    if app.mods.set_aside.is_empty() {
        return None;
    }
    let moved: Vec<String> = (app.mods.set_aside.iter())
        .map(|each| format!("`{}` to `{}`", each.name, each.to))
        .collect();
    Some(AppNotice {
        id: NOTICE.to_owned(),
        level: "info",
        text: format!(
            "Your edits to shipped mods were kept as copies: {}. Point a layout at a copy to use it.",
            moved.join(", ")
        ),
        action: None,
        dismiss: true,
        about: None,
    })
}

/// Takes the notice away. Returns whether `id` was it.
pub(crate) fn dismiss(app: &mut App, id: &str) -> bool {
    id == NOTICE && !std::mem::take(&mut app.mods.set_aside).is_empty()
}
