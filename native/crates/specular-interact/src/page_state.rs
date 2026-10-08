//! What each hosted page last said about itself: the live title, address,
//! load, history and scroll. Session state, so none of it is saved or undone,
//! apart from the address, which is also written to the page entity.

use std::collections::HashMap;

use glam::DVec2;
use specular_core::PageNav;
use specular_doc::{Command, EntityId, Kind, Page};

use crate::anchor::canonical_page_url;
use crate::app::page_of;
use crate::{Action, App, Effect, PageNotice};

/// The live state of one hosted page.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageState {
    /// The document's title. Empty until the page reports one, and for a
    /// document with none.
    pub title: String,
    /// The address the page shows, once it has reported one.
    pub url: Option<String>,
    /// Whether a load is in flight.
    pub loading: bool,
    /// Whether the page has an entry to go back to.
    pub can_go_back: bool,
    /// Whether the page has an entry to go forward to.
    pub can_go_forward: bool,
    /// How far the document is scrolled, in the page's CSS pixels.
    pub scroll: DVec2,
    /// The page's remote-debugging websocket, once the backend has one.
    pub devtools_url: Option<String>,
}

/// The live state of every hosted page that has reported anything.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct PageStates(HashMap<EntityId, PageState>);

impl PageStates {
    pub(crate) fn get(&self, page: &EntityId) -> Option<&PageState> {
        self.0.get(page)
    }

    /// Forgets the pages `keep` turns down.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&EntityId) -> bool) {
        self.0.retain(|page, _| keep(page));
    }
}

/// Records what `page` reported. Returns whether the document changed: a new
/// address is written to the page entity, with no undo step, so the file
/// reopens where the page was left.
pub(crate) fn on_notice(app: &mut App, page: &EntityId, notice: &PageNotice) -> bool {
    if app.document.entity(page).and_then(page_of).is_none() {
        return false;
    }
    let state = app.session.pages.0.entry(page.clone()).or_default();
    match notice {
        PageNotice::Title(title) => state.title.clone_from(title),
        PageNotice::Loading {
            loading,
            can_go_back,
            can_go_forward,
        } => {
            state.loading = *loading;
            state.can_go_back = *can_go_back;
            state.can_go_forward = *can_go_forward;
        }
        PageNotice::Scrolled { x, y } => state.scroll = DVec2::new(*x, *y),
        PageNotice::DevtoolsUrl(url) => state.devtools_url = Some(url.clone()),
        PageNotice::Url(url) => {
            if state.url.as_ref() != Some(url) {
                // Another document starts at its top, and a page tells of a
                // scroll only when it moves. A hash change is the same
                // document, scrolled where it was.
                let document = |url: &str| canonical_page_url(url);
                if state.url.as_deref().and_then(document) != document(url) {
                    state.scroll = DVec2::ZERO;
                }
                state.url = Some(url.clone());
                return write_url(app, page, url);
            }
        }
        PageNotice::Crashed { .. } => state.loading = false,
        PageNotice::Loaded { .. } | PageNotice::ImeCompositionBounds(_) => {}
    }
    false
}

/// Makes `url` the page entity's address, outside the history. Returns
/// whether that changed it.
fn write_url(app: &mut App, page: &EntityId, url: &str) -> bool {
    let Some(current) = app.document.entity(page).and_then(page_of) else {
        return false;
    };
    if current.url == url {
        return false;
    }
    let kind = Kind::Page(Page {
        url: url.to_owned(),
        ..current.clone()
    });
    let command = Command::SetKind {
        id: page.clone(),
        kind: Box::new(kind),
    };
    match app.document.apply(command) {
        Ok(_) => true,
        Err(error) => {
            tracing::warn!("page address not recorded: {error}");
            false
        }
    }
}

/// The page a navigation action is for: the entered page, or the page that
/// is the whole selection.
pub(crate) fn target(app: &App) -> Option<&EntityId> {
    let session = &app.session;
    let page = session.focus.page().or(session.selection.single_entity())?;
    app.document.entity(page).and_then(page_of).map(|_| page)
}

/// Runs a navigation action on its target. Going back or forward does
/// nothing where the page has said there is nowhere to go.
pub(crate) fn navigate(app: &App, action: &Action, effects: &mut Vec<Effect>) {
    let Some(page) = target(app) else {
        return;
    };
    let state = app.session.pages.get(page);
    let can = |allowed: fn(&PageState) -> bool| state.is_some_and(allowed);
    let nav = match action {
        Action::PageBack if can(|state| state.can_go_back) => PageNav::Back,
        Action::PageForward if can(|state| state.can_go_forward) => PageNav::Forward,
        Action::PageReload => PageNav::Reload,
        Action::PageStop if can(|state| state.loading) => PageNav::Stop,
        _ => return,
    };
    effects.push(Effect::Navigate {
        page: page.clone(),
        nav,
    });
}

/// Whether `nav` can be asked of `page`: it is a page, and for a step back
/// or forward it has said there is an entry there. The reason, when not.
pub(crate) fn allows(app: &App, page: &EntityId, nav: &PageNav) -> Result<(), String> {
    if app.document.entity(page).and_then(page_of).is_none() {
        return Err(format!("{} is not a page", page.as_str()));
    }
    let state = app.session.pages.get(page);
    match nav {
        PageNav::Back if !state.is_some_and(|state| state.can_go_back) => Err(format!(
            "{} has no earlier entry to go back to",
            page.as_str()
        )),
        PageNav::Forward if !state.is_some_and(|state| state.can_go_forward) => Err(format!(
            "{} has no later entry to go forward to",
            page.as_str()
        )),
        PageNav::Back | PageNav::Forward | PageNav::To(_) | PageNav::Reload | PageNav::Stop => {
            Ok(())
        }
    }
}
