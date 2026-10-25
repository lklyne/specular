//! The repo row of a single page.

use specular_agent::origin_of;
use specular_doc::Entity;

use super::super::super::{
    Control, ControlId, Dropdown, DropdownOption, DropdownSection, Face, Icon, OptionLayout,
};
use crate::{Action, App, RepoAction};

/// `/Users/<name>` shortened to `~`.
fn tilde(path: &str) -> String {
    match path.strip_prefix("/Users/") {
        Some(rest) => match rest.split_once('/') {
            Some((_, tail)) => format!("~/{tail}"),
            None => "~".to_owned(),
        },
        None => path.to_owned(),
    }
}

/// The most characters of a folder the repo row shows, so the path stays
/// clear of the row's trailing text.
const FOLDER_CHARS: usize = 28;

/// A folder for the repo row: `~` for the home folder, and the end of a long
/// path after an ellipsis, since the end is what tells two repos apart.
fn short_folder(path: &str) -> String {
    let path = tilde(path);
    let length = path.chars().count();
    if length <= FOLDER_CHARS {
        return path;
    }
    let tail: String = path.chars().skip(length - (FOLDER_CHARS - 1)).collect();
    format!("\u{2026}{tail}")
}

/// Which repo the page's origin writes to. A page with no origin (`data:`,
/// `file:`) has nothing to bind and no control.
pub(super) fn repo_control(app: &App, page: &Entity) -> Option<Control> {
    let live = app
        .page_state(&page.id)
        .and_then(|state| state.url.as_deref());
    let stored = app.page(&page.id);
    let origin = live
        .and_then(origin_of)
        .or_else(|| stored.and_then(|page| origin_of(&page.url)))?;
    let bound = app
        .repos()
        .binding(&origin)
        .map(|binding| binding.repo_path);
    let id = ControlId::new("page.repo");
    let pick = Action::Repo(RepoAction::Pick(Some(origin.clone())));
    let row = |name: &str, label: String, trailing: Option<&str>, enabled, action| DropdownOption {
        id: id.child(name),
        face: Face::text(label.clone()),
        label: label.into(),
        trailing: trailing.map(|text| text.to_owned().into()),
        fields: Vec::new(),
        chord: None,
        selected: false,
        enabled,
        action,
    };
    let mut options = vec![
        row("origin", origin.clone(), None, false, pick.clone()),
        match bound {
            Some(path) => row(
                "folder",
                short_folder(path),
                Some("Change\u{2026}"),
                true,
                pick,
            ),
            None => row(
                "folder",
                "No repo linked".to_owned(),
                Some("Choose\u{2026}"),
                true,
                pick,
            ),
        },
    ];
    if bound.is_some() {
        let unlink = Action::Repo(RepoAction::Unlink(origin));
        options.push(row("unlink", "Unlink".to_owned(), None, true, unlink));
    }
    Some(Control::Dropdown(Dropdown {
        label: match bound {
            Some(path) => format!("Repo: {}", tilde(path)).into(),
            None => "Link a repo".into(),
        },
        summary: Face::icon(Icon::Repo),
        content: vec![DropdownSection::Options {
            layout: OptionLayout::List,
            options,
        }],
        id,
    }))
}
