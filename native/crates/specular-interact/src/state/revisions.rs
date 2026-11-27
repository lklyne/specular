//! A revision per key, which moves when the key's value does.
//!
//! Whoever re-renders from the keyed state keeps one: it publishes every key
//! after a change and redraws only what read a key whose revision moved, so
//! a change to the dock leaves the toolbar alone.

use std::collections::BTreeMap;

use serde_json::Value;

/// Every published key with its value and the revision it is at.
#[derive(Debug, Default)]
pub struct Revisions {
    keys: BTreeMap<String, (u64, Value)>,
    clock: u64,
}

impl Revisions {
    /// Sets `key` to `value`. Returns whether it changed, which is when its
    /// revision moved.
    pub fn publish(&mut self, key: &str, value: Value) -> bool {
        if let Some((_, old)) = self.keys.get(key)
            && *old == value
        {
            return false;
        }
        self.clock += 1;
        self.keys.insert(key.to_owned(), (self.clock, value));
        true
    }

    /// The value of `key`, if it was ever published.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.keys.get(key).map(|(_, value)| value)
    }

    /// The revision of `key`, or 0 for a key never published.
    pub fn revision(&self, key: &str) -> u64 {
        self.keys.get(key).map_or(0, |(revision, _)| *revision)
    }

    /// A count that moves whenever any key's value does: a reader that saw
    /// the same count last time has nothing new to look at.
    pub const fn clock(&self) -> u64 {
        self.clock
    }

    /// The published keys, in order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.keys.keys().map(String::as_str)
    }

    /// Every key and its value: what `specular mod state` writes and what
    /// `specular mod check` reads back as a fixture.
    pub fn dump(&self) -> Value {
        Value::Object(
            (self.keys.iter())
                .map(|(key, (_, value))| (key.clone(), value.clone()))
                .collect(),
        )
    }
}
