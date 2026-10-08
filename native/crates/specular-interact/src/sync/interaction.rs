//! Interaction sync (ADR 0030): the entered page's hovers and clicks are
//! replayed on the pages of its sync set, each at its own element.
//!
//! Semantic, not positional. The entered page describes the element it was
//! pointed at; each peer lists its own elements; the match is scored here,
//! and input is replayed only on a confident one. An ambiguous or missing
//! match is skipped, since a click on the wrong element is worse than none.
//! Only the entered page is captured, so what is replayed on a peer is
//! never captured in turn.

use std::collections::HashMap;

use glam::Vec2;
use specular_core::{
    LocatorBundle, LocatorCandidate, LocatorResolution, PointKind, resolve_locator,
};
use specular_doc::EntityId;

use super::{address, peers};
use crate::{App, Effect};

/// What a peer was asked, by kind. A newer question of a kind replaces the
/// older one, and an answer counts only for the question still standing. A
/// hover and a click are kept apart so that the hover a frame after a click
/// cannot drop the click's answer.
#[derive(Debug, Clone, PartialEq, Default)]
struct Asked {
    hover: Option<(u64, LocatorBundle)>,
    click: Option<(u64, LocatorBundle)>,
}

/// What interaction sync remembers between events.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Interaction {
    /// The page whose input is captured: the entered page, while it is in a
    /// sync set.
    capturing: Option<EntityId>,
    asked: HashMap<EntityId, Asked>,
    requests: u64,
    /// Until when each peer may be on its way somewhere by a click that
    /// was replayed on it, on the session's clock.
    clicked_until: HashMap<EntityId, u64>,
}

/// How long a replayed click is given to take its page somewhere, in
/// milliseconds, before the page is sent there instead.
const CLICK_SETTLE_MS: u64 = 1000;

impl Interaction {
    /// Whether a click replayed on `peer` may still be navigating it.
    pub(crate) fn is_following_a_click(&self, peer: &EntityId, now_ms: u64) -> bool {
        (self.clicked_until.get(peer)).is_some_and(|until| *until > now_ms)
    }
}

/// The origin of an `http` or `https` address. Every other address has an
/// opaque origin (`file:`, `data:`, `about:blank`), and opaque documents
/// are not each other's peers.
fn origin(url: &str) -> Option<&str> {
    let (scheme, rest) = url.split_once("://")?;
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    let host = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    url.get(..scheme.len() + 3 + host)
}

/// Keeps the captured page the entered page of a sync set, and tells the
/// shell when that changes.
pub(crate) fn refresh_capture(app: &mut App, effects: &mut Vec<Effect>) {
    let wanted = (app.session.focus.page())
        .filter(|page| app.is_synced(page))
        .cloned();
    let state = &mut app.session.sync.interaction;
    if state.capturing != wanted {
        state.asked.clear();
        state.capturing.clone_from(&wanted);
        effects.push(Effect::CapturePage(wanted));
    }
}

/// The captured page was pointed at: asks each peer on the same origin for
/// the elements the bundle could mean.
pub(crate) fn on_pointed(
    app: &mut App,
    page: &EntityId,
    kind: PointKind,
    bundle: &LocatorBundle,
    effects: &mut Vec<Effect>,
) {
    if app.session.sync.interaction.capturing.as_ref() != Some(page) {
        return;
    }
    let here = address(app, page);
    let Some(here) = here.as_deref().and_then(origin) else {
        return;
    };
    for peer in peers(&app.document, page) {
        if address(app, &peer).as_deref().and_then(origin) != Some(here) {
            continue;
        }
        let state = &mut app.session.sync.interaction;
        state.requests += 1;
        let request = state.requests;
        let asked = state.asked.entry(peer.clone()).or_default();
        match kind {
            PointKind::Hover => asked.hover = Some((request, bundle.clone())),
            // A click outranks the hover before it, whose late answer must
            // not move the pointer off the element after the click lands.
            PointKind::Click => {
                asked.click = Some((request, bundle.clone()));
                asked.hover = None;
            }
        }
        effects.push(Effect::AskCandidates {
            page: peer,
            request,
            bundle: Box::new(bundle.clone()),
        });
    }
}

/// A peer answered with its elements: replays the input at its own element
/// when exactly one is a confident match, and does nothing otherwise.
pub(crate) fn on_candidates(
    app: &mut App,
    peer: &EntityId,
    request: u64,
    candidates: &[LocatorCandidate],
    effects: &mut Vec<Effect>,
) {
    let Some(asked) = app.session.sync.interaction.asked.get_mut(peer) else {
        return;
    };
    let standing = |slot: &Option<(u64, LocatorBundle)>| {
        slot.as_ref().is_some_and(|(asked, _)| *asked == request)
    };
    let (kind, slot) = if standing(&asked.hover) {
        (PointKind::Hover, asked.hover.take())
    } else if standing(&asked.click) {
        (PointKind::Click, asked.click.take())
    } else {
        return;
    };
    let Some((_, bundle)) = slot else {
        return;
    };
    match resolve_locator(&bundle, candidates) {
        LocatorResolution::Confident { point, .. } => {
            if kind == PointKind::Click {
                let until = app.session.now_ms + CLICK_SETTLE_MS;
                let state = &mut app.session.sync.interaction;
                state.clicked_until.insert(peer.clone(), until);
            }
            effects.push(Effect::ReplayPointer {
                page: peer.clone(),
                kind,
                point: Vec2::new(point.0 as f32, point.1 as f32),
            });
        }
        LocatorResolution::Ambiguous | LocatorResolution::None => {}
    }
}
