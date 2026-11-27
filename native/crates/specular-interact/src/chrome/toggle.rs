//! What a person hides and shows at run time over the layout file, and the
//! one key that sets it all aside.
//!
//! A toggle is kept apart from the layout file (the shell writes it under
//! `layouts/.state/`), so a click never rewrites a file a person edits.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{ChromeState, LayoutError};
use crate::{App, Effect, Key, KeyInput};

/// The reset key as macOS writes it: Control, Option, Shift and Escape.
pub const RESET_KEYS: &str = "⌃⌥⇧⎋";

/// Whether `input` is the reset key, which no binding can take: Control,
/// Option, Shift and Escape, and not Command.
pub(crate) fn is_reset(input: &KeyInput) -> bool {
    let held = input.modifiers;
    input.key == Key::Escape && held.control && held.alt && held.shift && !held.meta
}

/// The regions and controls hidden or shown at run time, over what the
/// layout says. An id the layout already has as it was toggled is not kept.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Toggles {
    /// Each toggled id, region or control, and whether it is hidden.
    #[serde(default)]
    pub hidden: BTreeMap<String, bool>,
}

impl Toggles {
    /// The toggles `text` says, or why it says none.
    pub fn parse(text: &str) -> Result<Self, LayoutError> {
        Ok(serde_json::from_str(text)?)
    }

    /// The toggles as their file holds them.
    pub fn to_text(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    /// Whether `id` was toggled hidden (`Some(true)`) or shown.
    pub fn get(&self, id: &str) -> Option<bool> {
        self.hidden.get(id).copied()
    }
}

/// A run-time change to what is drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutAction {
    /// Hide the region or control `id` if it is shown, show it if hidden.
    Toggle(String),
    /// Show it.
    Show(String),
    /// Hide it.
    Hide(String),
    /// Show the layout named, or with none the next one by name.
    Switch(Option<String>),
    /// Set the toggles aside and go back to the shipped layout.
    Reset,
}

impl ChromeState {
    /// Hides or shows `id`. Returns whether that changed anything.
    fn set(&mut self, id: String, hidden: bool) -> bool {
        if self.hidden(&id) == hidden {
            return false;
        }
        if self.layout().hides(&id) == hidden {
            self.toggles_mut().hidden.remove(&id);
        } else {
            self.toggles_mut().hidden.insert(id, hidden);
        }
        true
    }

    /// The layout `name` names, or the one after the one in force.
    fn next(&self, name: Option<&str>) -> Option<String> {
        if let Some(name) = name {
            return self.layouts.contains_key(name).then(|| name.to_owned());
        }
        let names: Vec<&String> = self.layouts.keys().collect();
        let at = names.iter().position(|name| **name == self.active);
        let next = at.map_or(0, |at| (at + 1) % names.len().max(1));
        names.get(next).map(|name| (*name).clone())
    }
}

/// Runs `action`, asking the shell to keep the toggles or to reset its
/// files.
pub(crate) fn run(app: &mut App, action: LayoutAction, effects: &mut Vec<Effect>) {
    let chrome = &mut app.chrome;
    let changed = match action {
        LayoutAction::Toggle(id) => {
            let hidden = !chrome.hidden(&id);
            chrome.set(id, hidden)
        }
        LayoutAction::Show(id) => chrome.set(id, false),
        LayoutAction::Hide(id) => chrome.set(id, true),
        LayoutAction::Switch(name) => {
            if let Some(next) = chrome.next(name.as_deref())
                && next != chrome.active
            {
                chrome.active.clone_from(&next);
                effects.push(Effect::SaveActiveLayout(next));
            }
            return;
        }
        LayoutAction::Reset => {
            chrome.toggles.clear();
            super::state::DEFAULT.clone_into(&mut chrome.active);
            effects.push(Effect::ResetLayout);
            return;
        }
    };
    if changed {
        let toggles = chrome.toggles().clone();
        effects.push(Effect::SaveLayoutToggles(chrome.active.clone(), toggles));
    }
}
