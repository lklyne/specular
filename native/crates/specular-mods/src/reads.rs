//! Narrow re-renders: which keys each mod read.
//!
//! A key's revision moves when its value does ([`Revisions::publish`]). While
//! a mod renders, every key it reads is recorded against it ([`Reads`]). A mod
//! is rendered again only when a key it read has moved since, so a change to
//! the dock leaves the toolbar's script alone. A value shared between mods is
//! a key too, under `shared/`.

use std::collections::BTreeMap;

use specular_interact::Revisions;

/// The keys each mod read in its last render, with the revision each was at.
#[derive(Debug, Default)]
pub struct Reads {
    by_mod: BTreeMap<String, BTreeMap<String, u64>>,
}

impl Reads {
    /// `id` is about to render: what it read last time is forgotten.
    pub fn begin(&mut self, id: &str) {
        self.by_mod.entry(id.to_owned()).or_default().clear();
    }

    /// `id` read `key` at `revision`. Reads made outside a render, from an
    /// event handler, are kept too: a handler that looked at a key acts on it.
    pub fn record(&mut self, id: &str, key: &str, revision: u64) {
        (self.by_mod.entry(id.to_owned()).or_default()).insert(key.to_owned(), revision);
    }

    /// Whether a key `id` read has moved since it read it. A mod that has
    /// read nothing yet is stale, so its first render happens.
    pub fn stale(&self, id: &str, revisions: &Revisions) -> bool {
        match self.by_mod.get(id) {
            None => true,
            Some(read) => (read.iter()).any(|(key, seen)| revisions.revision(key) != *seen),
        }
    }

    /// The keys `id` read, for `specular mod stats`.
    pub fn read_by(&self, id: &str) -> Vec<String> {
        (self.by_mod.get(id)).map_or_else(Vec::new, |read| read.keys().cloned().collect())
    }

    /// Forgets `id`, when it is unloaded.
    pub fn forget(&mut self, id: &str) {
        self.by_mod.remove(id);
    }
}
