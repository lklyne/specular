//! Sync sets (ADR 0027): pages that share a `syncId` navigate and scroll
//! together, whatever group each is in. This is the multi-breakpoint
//! workflow: one site at several widths, driven as one.
//!
//! Membership is in the document, so joining and leaving are undo steps.
//! Following is session state. A navigation is followed from any page of
//! the set, and a page that was just sent somewhere keeps quiet for a
//! moment so that its own arrival is not sent back. A scroll is followed
//! only from the entered page, the one page real input reaches, so a
//! follower's scroll can never come back. Hovers and clicks are followed
//! the same way, by [`interaction`].

pub(crate) mod interaction;

use std::collections::HashMap;

use glam::Vec2;
use specular_core::PageNav;
use specular_doc::{Command, Document, EntityId, Kind, Page};

use crate::app::page_of;
use crate::{App, Effect, PageNotice, update};

/// How long a page that was sent somewhere says nothing of where it goes,
/// in milliseconds: long enough for the load to commit and for a redirect
/// that only one width takes to settle.
const NAVIGATION_QUIET_MS: u64 = 1500;

/// What following needs to remember between events.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct SyncState {
    /// Until when each page's navigations are its own business, on the
    /// session's clock.
    quiet_until: HashMap<EntityId, u64>,
    /// Where each peer is owed a navigation to, once the click that was
    /// replayed on it has had its chance to take it there.
    owed: HashMap<EntityId, String>,
    /// Which page is captured and what its peers were asked.
    pub(crate) interaction: interaction::Interaction,
}

impl SyncState {
    /// Forgets the pages `keep` turns down.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&EntityId) -> bool) {
        self.quiet_until.retain(|page, _| keep(page));
        self.owed.retain(|page, _| keep(page));
    }
}

fn sync_id<'a>(document: &'a Document, page: &EntityId) -> Option<&'a str> {
    page_of(document.entity(page)?)?.sync_id.as_deref()
}

/// The other pages of `page`'s sync set, back to front.
fn peers(document: &Document, page: &EntityId) -> Vec<EntityId> {
    let Some(set) = sync_id(document, page) else {
        return Vec::new();
    };
    (document.entities())
        .filter(|entity| entity.id != *page)
        .filter(|entity| page_of(entity).is_some_and(|peer| peer.sync_id.as_deref() == Some(set)))
        .map(|entity| entity.id.clone())
        .collect()
}

impl App {
    /// Whether `page` is in a sync set with another page. An id no other
    /// page carries syncs with nothing, so it reads as unsynced.
    pub fn is_synced(&self, page: &EntityId) -> bool {
        !peers(&self.document, page).is_empty()
    }

    /// Whether the selected pages are all in one sync set, so that
    /// [`Action::ToggleSync`](crate::Action::ToggleSync) would take them
    /// out of it. `None` when toggling would do nothing: something other
    /// than pages is selected, or one page that is in no set.
    pub fn selection_synced(&self) -> Option<bool> {
        let pages = selected_pages(self)?;
        match pages.as_slice() {
            [] => None,
            [page] => self.is_synced(page).then_some(true),
            [first, rest @ ..] => {
                let set = sync_id(&self.document, first);
                Some(set.is_some() && rest.iter().all(|page| sync_id(&self.document, page) == set))
            }
        }
    }
}

/// The selection, when it is pages and nothing else.
fn selected_pages(app: &App) -> Option<Vec<EntityId>> {
    let selection = &app.session.selection;
    let pages: Vec<EntityId> = (selection.entities())
        .filter(|id| app.document.entity(id).and_then(page_of).is_some())
        .cloned()
        .collect();
    (pages.len() == selection.items().len()).then_some(pages)
}

/// Toggle-merge, as one undo step: selected pages that share one set leave
/// it, and any other selection of two or more becomes one new set. A set
/// left with one page is dissolved in the same step.
pub(crate) fn toggle(app: &mut App, effects: &mut Vec<Effect>) {
    let Some(leave) = app.selection_synced() else {
        return;
    };
    let Some(selected) = selected_pages(app) else {
        return;
    };
    let joined = (!leave).then(|| format!("sync_{}", app.fresh_id()));
    let mut after: Vec<(EntityId, Page, Option<String>)> = (app.document.entities())
        .filter_map(|entity| {
            let page = page_of(entity)?;
            let set = if selected.contains(&entity.id) {
                joined.clone()
            } else {
                page.sync_id.clone()
            };
            Some((entity.id.clone(), page.clone(), set))
        })
        .collect();
    let mut members: HashMap<String, usize> = HashMap::new();
    for set in after.iter().filter_map(|(_, _, set)| set.clone()) {
        *members.entry(set).or_default() += 1;
    }
    for (_, _, set) in &mut after {
        if (set.as_ref()).is_some_and(|set| members.get(set).copied().unwrap_or(0) < 2) {
            *set = None;
        }
    }
    let commands: Vec<Command> = after
        .into_iter()
        .filter(|(_, page, set)| page.sync_id != *set)
        .map(|(id, page, sync_id)| Command::SetKind {
            id,
            kind: Box::new(Kind::Page(Page { sync_id, ..page })),
        })
        .collect();
    if !commands.is_empty() {
        update::document_step(app, Command::Batch(commands), effects);
    }
}

/// The address `page` shows: what it last reported, or what the document
/// has for a page that has reported nothing.
fn address(app: &App, page: &EntityId) -> Option<String> {
    (app.page_state(page).and_then(|state| state.url.clone()))
        .or_else(|| Some(page_of(app.document.entity(page)?)?.url.clone()))
        .filter(|url| !url.is_empty())
}

fn hush(app: &mut App, page: &EntityId) {
    let until = app.session.now_ms + NAVIGATION_QUIET_MS;
    app.session.sync.quiet_until.insert(page.clone(), until);
}

fn is_quiet(app: &App, page: &EntityId) -> bool {
    (app.session.sync.quiet_until.get(page)).is_some_and(|until| *until > app.session.now_ms)
}

/// `page` was sent through `nav` by the user or the API: sends its sync set
/// the same way. A peer with no entry to step to, or showing another
/// address, is loaded at the address `page` was showing instead.
pub(crate) fn on_driven(app: &mut App, page: &EntityId, nav: &PageNav, effects: &mut Vec<Effect>) {
    let peers = peers(&app.document, page);
    if peers.is_empty() || *nav == PageNav::Stop {
        return;
    }
    let here = address(app, page);
    hush(app, page);
    for peer in peers {
        hush(app, &peer);
        let there = address(app, &peer);
        let state = app.page_state(&peer);
        let fallback = || here.clone().map(PageNav::To);
        let step = match nav {
            PageNav::To(url) => (there.as_deref() != Some(url)).then(|| nav.clone()),
            PageNav::Back if state.is_some_and(|state| state.can_go_back) => Some(PageNav::Back),
            PageNav::Forward if state.is_some_and(|state| state.can_go_forward) => {
                Some(PageNav::Forward)
            }
            PageNav::Reload if there == here => Some(PageNav::Reload),
            PageNav::Back | PageNav::Forward | PageNav::Reload => fallback(),
            PageNav::Stop => None,
        };
        if let Some(nav) = step {
            effects.push(Effect::Navigate { page: peer, nav });
        }
    }
}

/// `page` reported something. `shown` is the address it showed until now.
pub(crate) fn on_notice(
    app: &mut App,
    page: &EntityId,
    notice: &PageNotice,
    shown: Option<&str>,
    effects: &mut Vec<Effect>,
) {
    let entered = app.session.focus.page() == Some(page);
    match notice {
        // The first address a page reports is its own load, not a move.
        PageNotice::Url(url) if shown.is_some_and(|shown| shown != url) => {
            on_navigated(app, page, url, effects);
        }
        PageNotice::Scrolled { .. } if entered && app.is_synced(page) => {
            effects.push(Effect::AskScrollProgress(page.clone()));
        }
        PageNotice::ScrollProgress { x, y } if entered => {
            let progress = Vec2::new(*x as f32, *y as f32);
            for page in peers(&app.document, page) {
                effects.push(Effect::ScrollPage { page, progress });
            }
        }
        PageNotice::Pointed { kind, bundle } => {
            interaction::on_pointed(app, page, *kind, bundle, effects);
        }
        PageNotice::Candidates {
            request,
            candidates,
        } => interaction::on_candidates(app, page, *request, candidates, effects),
        PageNotice::Url(_)
        | PageNotice::Scrolled { .. }
        | PageNotice::ScrollProgress { .. }
        | PageNotice::Loaded { .. }
        | PageNotice::Crashed { .. }
        | PageNotice::ImeCompositionBounds(_)
        | PageNotice::Title(_)
        | PageNotice::Favicon(_)
        | PageNotice::Loading { .. }
        | PageNotice::Inspected { .. }
        | PageNotice::ElementCaptured { .. }
        | PageNotice::ElementPlaces(_)
        | PageNotice::DevtoolsUrl(_) => {}
    }
}

/// `page` went to `url` by itself: a link, a redirect, a script, or a
/// change of hash. Its peers are sent there, and a peer already there is
/// left alone. A change of hash is loaded like any address, which a
/// browser turns into a move within the document it has.
fn on_navigated(app: &mut App, page: &EntityId, url: &str, effects: &mut Vec<Effect>) {
    if is_quiet(app, page) {
        return;
    }
    for peer in peers(&app.document, page) {
        hush(app, &peer);
        // A link clicked on `page` was clicked on the peer too. The peer
        // is given the time to follow it, so that it loads once.
        let now = app.session.now_ms;
        if (app.session.sync.interaction).is_following_a_click(&peer, now) {
            app.session.sync.owed.insert(peer, url.to_owned());
            continue;
        }
        if address(app, &peer).as_deref() != Some(url) {
            effects.push(Effect::Navigate {
                page: peer,
                nav: PageNav::To(url.to_owned()),
            });
        }
    }
}

/// The clock moved: a peer whose replayed click did not take it where its
/// set went is sent there.
pub(crate) fn on_tick(app: &mut App, effects: &mut Vec<Effect>) {
    if app.session.sync.owed.is_empty() {
        return;
    }
    let now = app.session.now_ms;
    let state = &app.session.sync;
    let due: Vec<EntityId> = (state.owed.keys())
        .filter(|peer| !state.interaction.is_following_a_click(peer, now))
        .cloned()
        .collect();
    for peer in due {
        let Some(url) = app.session.sync.owed.remove(&peer) else {
            continue;
        };
        if address(app, &peer).as_deref() != Some(url.as_str()) {
            hush(app, &peer);
            effects.push(Effect::Navigate {
                page: peer,
                nav: PageNav::To(url),
            });
        }
    }
}
