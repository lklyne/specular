//! The dock's controls for pages (`PagePopup.tsx`): history, reload and the address
//! (several pages keep the address alone), then the size list, which also holds the device frame, orientation and
//! color scheme, and the chain button that syncs the selected pages, and the
//! repo control of a single page. Electron also infers a repo from a running dev server's base URL;
//! this app runs no dev servers, so the origin's binding is the only source.

mod repo;
mod size;

use self::repo::repo_control;
pub(super) use self::size::preset_sections;
use self::size::size_dropdown;

use specular_doc::Entity;

use super::super::build::groups;
use super::super::{
    Button, Control, ControlId, ControlsModel, Face, Field, FieldSubmit, FieldWidth, Icon, PAGE_URL,
};
use super::actions::Actions;
use crate::{Action, App, binding_of};

/// A button of the history row. It cannot be pressed where the page has said
/// there is nowhere to go.
fn history_button(
    id: &'static str,
    label: &'static str,
    icon: Icon,
    action: Action,
    enabled: bool,
) -> Control {
    Control::Button(Button {
        id: ControlId::new(id),
        label: label.into(),
        face: Face::icon(icon),
        enabled,
        chord: binding_of(&action).map(|binding| binding.chord),
        action,
    })
}

/// Back, forward, and reload, which is stop while the page loads.
fn history(app: &App, page: &Entity) -> Vec<Control> {
    let state = app.page_state(&page.id);
    let can = |allowed: fn(&crate::PageState) -> bool| state.is_some_and(allowed);
    let (loading, back, forward) = (
        can(|state| state.loading),
        can(|state| state.can_go_back),
        can(|state| state.can_go_forward),
    );
    let load = if loading {
        history_button(
            "page.reload",
            "Stop loading",
            Icon::Stop,
            Action::PageStop,
            true,
        )
    } else {
        history_button(
            "page.reload",
            "Reload",
            Icon::Reload,
            Action::PageReload,
            true,
        )
    };
    vec![
        history_button(
            "page.back",
            "Back",
            Icon::ChevronLeft,
            Action::PageBack,
            back,
        ),
        history_button(
            "page.forward",
            "Forward",
            Icon::ChevronRight,
            Action::PageForward,
            forward,
        ),
        load,
    ]
}

/// The address `page` shows, or nothing for a page that has none yet.
fn shown(app: &App, page: &Entity) -> String {
    let live = app.page_state(&page.id).and_then(|state| state.url.clone());
    let stored = || app.page(&page.id).map(|page| page.url.clone());
    let url = live.or_else(stored).unwrap_or_default();
    if url == "about:blank" {
        String::new()
    } else {
        url
    }
}

/// The address field of `pages`. It holds the address they all show, and is
/// empty where they differ. What is typed takes every one of them there.
fn address(app: &App, pages: &[&Entity]) -> Control {
    let mut shown = pages.iter().map(|page| shown(app, page));
    let first = shown.next().unwrap_or_default();
    let shared = shown.all(|url| url == first);
    Control::Field(Field {
        id: ControlId::new(PAGE_URL),
        label: "Page address".into(),
        caption: None,
        value: if shared { first } else { String::new() },
        placeholder: Some(
            if shared {
                "Search or type a URL"
            } else {
                "Type a URL for every page"
            }
            .into(),
        ),
        width: FieldWidth::Wide,
        submit: FieldSubmit::PageUrl,
    })
}

pub(super) fn controls(app: &App, entities: &[&Entity]) -> ControlsModel {
    let single = match entities {
        [one] => Some(*one),
        _ => None,
    };
    let controls = if let Some(page) = single {
        groups(vec![
            history(app, page),
            vec![address(app, entities)],
            vec![size_dropdown(app, single)],
            repo_control(app, page).into_iter().collect(),
            Actions {
                sync: app.selection_synced(),
                annotate: false,
                ..Actions::all("page", 1)
            }
            .controls(),
        ])
    } else {
        groups(vec![
            vec![address(app, entities)],
            vec![size_dropdown(app, None)],
            Actions {
                sync: app.selection_synced(),
                ..Actions::all(&format!("{} pages", entities.len()), entities.len())
            }
            .controls(),
        ])
    };
    ControlsModel { controls }
}
