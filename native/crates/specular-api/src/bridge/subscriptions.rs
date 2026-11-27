//! Which pages subscribed to which keys, and what to push them when a key
//! moves.

use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;

use specular_interact::{App, Revisions, state};

use super::push_script;

/// Every page's subscribed keys, and the values they were last given.
#[derive(Debug)]
pub struct Subscriptions<P> {
    pages: HashMap<P, BTreeSet<&'static str>>,
    /// The subscribed keys only, so a key no page reads never moves.
    revisions: Revisions,
}

impl<P> Default for Subscriptions<P> {
    fn default() -> Self {
        Self {
            pages: HashMap::new(),
            revisions: Revisions::default(),
        }
    }
}

impl<P: Clone + Eq + Hash> Subscriptions<P> {
    /// `page` hears of `key` from now on; it was answered with `value`.
    pub fn subscribe(&mut self, page: P, key: &'static str, value: serde_json::Value) {
        // While others hear of the key, a move they have not been pushed
        // yet must still reach them, so only a key nobody reads is reset.
        if !self.pages.values().any(|keys| keys.contains(key)) {
            self.revisions.publish(key, value);
        }
        self.pages.entry(page).or_default().insert(key);
    }

    /// `page` no longer hears of `key`.
    pub fn unsubscribe(&mut self, page: &P, key: &str) {
        if let Some(keys) = self.pages.get_mut(page) {
            keys.remove(key);
            if keys.is_empty() {
                self.pages.remove(page);
            }
        }
    }

    /// `page` lost its bridge or closed: it hears of nothing.
    pub fn forget(&mut self, page: &P) {
        self.pages.remove(page);
    }

    /// The scripts to run in each page for the subscribed keys that moved
    /// since the last call. Nothing is read from `app` while nobody
    /// subscribes.
    pub fn changes(&mut self, app: &App) -> Vec<(P, String)> {
        if self.pages.is_empty() {
            return Vec::new();
        }
        let read: BTreeSet<&'static str> = self.pages.values().flatten().copied().collect();
        let moved: Vec<(&'static str, String)> = (state(app).into_iter())
            .filter(|(key, _)| read.contains(key))
            .filter_map(|(key, value)| {
                let script = push_script(key, &value);
                self.revisions.publish(key, value).then_some((key, script))
            })
            .collect();
        let mut out = Vec::new();
        for (key, script) in moved {
            for (page, keys) in &self.pages {
                if keys.contains(key) {
                    out.push((page.clone(), script.clone()));
                }
            }
        }
        out
    }
}
