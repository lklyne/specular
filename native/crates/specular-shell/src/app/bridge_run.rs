//! The page bridge's transport (ADR 0055): putting it in each page whose
//! address is a file of the open space and taking it out of every other,
//! answering its calls, and pushing the keys a page subscribed to.
//!
//! What a call means is `specular_api::bridge`; which page may have it is
//! `specular_interact::bridge_allowed`, asked again for every address a
//! page shows and for every call, with the address the backend reports.

use std::collections::HashMap;

use specular_api::Api;
use specular_api::bridge::{BridgeStep, Subscriptions, install_script};
use specular_core::PageId;
use specular_interact::bridge_allowed;

use super::effects::warn_on_error;
use super::runtime::{Runtime, ShellWindow, unix_ms};

/// Every hosted page's side of the bridge.
pub(crate) struct Bridges {
    /// What calls are planned by: the HTTP API's routes, with ids and
    /// tickets of their own.
    api: Api,
    /// The install script each page was given, with the nonce its calls
    /// carry, kept for the page's life.
    scripts: HashMap<PageId, Script>,
    subscriptions: Subscriptions<PageId>,
}

struct Script {
    nonce: String,
    /// Whether the bridge is in the page now.
    open: bool,
}

impl Default for Bridges {
    fn default() -> Self {
        Self {
            api: Api::new(unix_ms().rotate_left(17)),
            scripts: HashMap::new(),
            subscriptions: Subscriptions::default(),
        }
    }
}

impl<W: ShellWindow> Runtime<W> {
    /// Whether a page showing `url` may have the bridge in this space.
    fn bridge_allowed(&self, url: &str) -> bool {
        (self.space.as_deref()).is_some_and(|root| bridge_allowed(url, root))
    }

    /// Puts the bridge in `page`, which shows `url` now, or takes it out.
    pub(super) fn check_bridge(&mut self, page: PageId, url: &str) {
        let allowed = self.bridge_allowed(url);
        let script = (self.bridges.scripts.entry(page)).or_insert_with(|| Script {
            nonce: crate::api::new_secret(),
            open: false,
        });
        if allowed {
            script.open = true;
            let install = install_script(&script.nonce);
            warn_on_error(self.source.set_bridge(page, Some(&install)));
        } else if std::mem::take(&mut script.open) {
            self.bridges.subscriptions.forget(&page);
            warn_on_error(self.source.set_bridge(page, None));
        }
    }

    /// `page` finished a load: a bridge it has is put in the document
    /// again, in case the document began before it was asked for.
    pub(super) fn bridge_loaded(&mut self, page: PageId) {
        if let Some(script) = self.bridges.scripts.get(&page).filter(|script| script.open) {
            let install = install_script(&script.nonce);
            warn_on_error(self.source.set_bridge(page, Some(&install)));
        }
    }

    /// `page` is no longer hosted.
    pub(super) fn bridge_closed(&mut self, page: PageId) {
        self.bridges.scripts.remove(&page);
        self.bridges.subscriptions.forget(&page);
    }

    /// Answers `page`'s call `payload`, made while its main frame showed
    /// `url`. A page without the bridge, or no longer at an address that
    /// may have it, is not answered.
    pub(super) fn answer_bridge(&mut self, page: PageId, url: &str, payload: &str) {
        let nonce = match self.bridges.scripts.get(&page) {
            Some(script) if script.open && self.bridge_allowed(url) => script.nonce.clone(),
            _ => {
                tracing::warn!(%page, url, "a bridge call from a page that may not have one");
                return;
            }
        };
        let script = match self.bridges.api.bridge(&self.app, payload, &nonce) {
            BridgeStep::Reply(script) => script,
            BridgeStep::Run { event, pending } => {
                let outcome = specular_api::Host::run(self, event);
                pending.finish(&self.app, outcome.as_ref())
            }
            BridgeStep::Subscribe { key, value, reply } => {
                self.bridges.subscriptions.subscribe(page, key, value);
                reply
            }
            BridgeStep::Unsubscribe { key, reply } => {
                self.bridges.subscriptions.unsubscribe(&page, &key);
                reply
            }
            BridgeStep::Ignore => return,
        };
        warn_on_error(self.source.bridge_send(page, &script));
    }

    /// Pushes every subscribed key that moved to the pages that read it.
    pub(super) fn push_bridge_changes(&mut self) {
        for (page, script) in self.bridges.subscriptions.changes(&self.app) {
            warn_on_error(self.source.bridge_send(page, &script));
        }
    }
}
