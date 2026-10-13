//! What each hosted page last said about itself: the live title, address,
//! load, history and scroll. Session state, so none of it is saved or undone,
//! apart from the address, which is also written to the page entity.

use std::collections::HashMap;
use std::sync::Arc;

use glam::DVec2;
use specular_core::{ElementPlace, PageNav};
use specular_doc::{Command, EntityId, Kind, Page};

use crate::anchor::canonical_page_url;
use crate::app::page_of;
use crate::{App, Cursor, Effect, PageNotice, sync};

/// How a document fits the screen of the device it is shown as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenFit {
    /// Laid out inside the screen's safe area, as a document is unless it
    /// asks otherwise.
    #[default]
    Inside,
    /// Over the whole screen, as its `viewport-fit=cover` asks.
    Covers,
}

/// The live state of one hosted page.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageState {
    /// The document's title. Empty until the page reports one, and for a
    /// document with none.
    pub title: String,
    /// The document's icon as a PNG, when it has one.
    pub favicon: Option<Arc<[u8]>>,
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
    /// The cursor the page asks for over what the pointer is on.
    pub cursor: Cursor,
    /// The page's remote-debugging websocket, once the backend has one.
    pub devtools_url: Option<String>,
    /// How the document fits its device's screen. Kept from one document
    /// to the next until the new one says, so a site that covers is not
    /// laid out twice a page.
    pub fit: ScreenFit,
    /// The colour the document has a browser paint round it. `None` until
    /// it says, and for a document that names none.
    pub surround: Option<[u8; 3]>,
    /// Where the elements anchored items follow sit in the document, by
    /// selector (ADR 0032). A selector that finds nothing has no entry.
    pub elements: HashMap<String, ElementPlace>,
}

/// The live state of every hosted page that has reported anything.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct PageStates(HashMap<EntityId, PageState>);

impl PageStates {
    pub(crate) fn get(&self, page: &EntityId) -> Option<&PageState> {
        self.0.get(page)
    }

    /// Notes where `page` has just said the element `selector` names is.
    pub(crate) fn place(&mut self, page: &EntityId, selector: &str, place: ElementPlace) {
        let state = self.0.entry(page.clone()).or_default();
        state.elements.insert(selector.to_owned(), place);
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
        // A page tells of the same icon again and again. Kept as it was,
        // whoever drew it has nothing to decode.
        PageNotice::Favicon(png) if state.favicon.as_deref() == png.as_deref() => {}
        PageNotice::Favicon(png) => state.favicon.clone_from(png),
        PageNotice::Loading {
            loading,
            can_go_back,
            can_go_forward,
        } => {
            state.loading = *loading;
            state.can_go_back = *can_go_back;
            state.can_go_forward = *can_go_forward;
        }
        PageNotice::Scrolled { x, y } => {
            let scroll = DVec2::new(*x, *y);
            // An element in a fixed or sticky container travels through the
            // document with the scroll, and the page says so only a moment
            // later. Moved here, what is on it does not lurch in between.
            let by = (scroll - state.scroll).as_vec2();
            for place in state.elements.values_mut() {
                if place.viewport_positioned {
                    place.doc += by;
                }
            }
            state.scroll = scroll;
        }
        PageNotice::ElementPlaces(places) => {
            for (selector, place) in places {
                match place {
                    Some(place) => state.elements.insert(selector.clone(), *place),
                    None => state.elements.remove(selector),
                };
            }
        }
        PageNotice::ElementCaptured { request, element } => {
            return crate::attach::on_captured(app, page, *request, element.as_ref());
        }
        PageNotice::Cursor(cursor) => state.cursor = *cursor,
        PageNotice::DevtoolsUrl(url) => state.devtools_url = Some(url.clone()),
        PageNotice::ViewportFit { covers, color } => {
            state.fit = if *covers {
                ScreenFit::Covers
            } else {
                ScreenFit::Inside
            };
            state.surround = *color;
        }
        PageNotice::Url(url) => {
            if state.url.as_ref() != Some(url) {
                // Another document starts at its top, and a page tells of a
                // scroll only when it moves. A hash change is the same
                // document, scrolled where it was.
                let document = |url: &str| canonical_page_url(url);
                if state.url.as_deref().and_then(document) != document(url) {
                    state.scroll = DVec2::ZERO;
                    state.elements.clear();
                    state.favicon = None;
                }
                state.url = Some(url.clone());
                return write_url(app, page, url);
            }
        }
        PageNotice::Crashed { .. } => state.loading = false,
        PageNotice::Loaded { .. }
        | PageNotice::ImeCompositionBounds(_)
        | PageNotice::ScrollProgress { .. }
        | PageNotice::ScrollRoom(_)
        | PageNotice::Pointed { .. }
        | PageNotice::Inspected { .. }
        | PageNotice::RegionElements(_)
        | PageNotice::Candidates { .. } => {}
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

/// Puts the caret in the address of the pages the dock holds the controls
/// of, unless a drag is in flight. With no such page there is no address.
pub(crate) fn edit_url(app: &mut App, effects: &mut Vec<Effect>) {
    if app.session.gesture.is_none() && !addressed(app).is_empty() {
        let field = crate::ControlId::new(crate::panel::PAGE_URL);
        crate::edit::focus_field(app, &field, effects);
    }
}

/// The page a navigation action is for: the entered page, or the page that
/// is the whole selection.
pub(crate) fn target(app: &App) -> Option<&EntityId> {
    let session = &app.session;
    let page = session.focus.page().or(session.selection.single_entity())?;
    app.document.entity(page).and_then(page_of).map(|_| page)
}

/// The pages a typed address is for: the target page, or every page of a
/// selection that is pages and nothing else.
pub(crate) fn addressed(app: &App) -> Vec<EntityId> {
    if let Some(page) = target(app) {
        return vec![page.clone()];
    }
    let selection = &app.session.selection;
    let is_page = |id: &EntityId| app.document.entity(id).and_then(page_of).is_some();
    let pages: Vec<EntityId> = selection.entities().cloned().collect();
    if pages.len() == selection.items().len() && pages.iter().all(is_page) {
        pages
    } else {
        Vec::new()
    }
}

/// Navigates the target page. Going back or forward does nothing where
/// the page has said there is nowhere to go. An address goes to every page
/// it is for.
pub(crate) fn navigate(app: &mut App, nav: PageNav, effects: &mut Vec<Effect>) {
    if matches!(nav, PageNav::To(_)) {
        let pages = addressed(app);
        // A page of the selection is sent there itself, so a sync set does
        // not send it again.
        let mut followers = Vec::new();
        for page in &pages {
            sync::on_driven(app, page, &nav, &mut followers);
            effects.push(Effect::Navigate {
                page: page.clone(),
                nav: nav.clone(),
            });
        }
        let mut sent = pages;
        for follower in followers {
            if let Effect::Navigate { page, .. } = &follower {
                if sent.contains(page) {
                    continue;
                }
                sent.push(page.clone());
            }
            effects.push(follower);
        }
        return;
    }
    let Some(page) = target(app) else {
        return;
    };
    let state = app.session.pages.get(page);
    let can = |allowed: fn(&PageState) -> bool| state.is_some_and(allowed);
    let allowed = match nav {
        PageNav::Back => can(|state| state.can_go_back),
        PageNav::Forward => can(|state| state.can_go_forward),
        PageNav::Stop => can(|state| state.loading),
        PageNav::Reload | PageNav::To(_) => true,
    };
    if !allowed {
        return;
    }
    drive(app, &page.clone(), nav, effects);
}

/// Sends `page` through `nav` and its sync set with it.
pub(crate) fn drive(app: &mut App, page: &EntityId, nav: PageNav, effects: &mut Vec<Effect>) {
    let mut followers = Vec::new();
    sync::on_driven(app, page, &nav, &mut followers);
    effects.push(Effect::Navigate {
        page: page.clone(),
        nav,
    });
    effects.append(&mut followers);
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
