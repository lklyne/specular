//! What a canvas's item tabs keep: which items have one, in what order, and
//! for each its lens. A save writes them beside the canvas's camera
//! ([`SavedTab`]), so they are there at the next launch.

use std::collections::HashMap;

use specular_doc::{Document, EntityId};

use super::can_show;

/// How a tab looks at its item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Lens {
    /// The item fills the area under the chrome, as a browser tab or an
    /// editor does.
    #[default]
    Fill,
    /// The item at its stored size, fitted into the window.
    Device,
}

impl Lens {
    /// Every lens, in the order the control lists them.
    pub const ALL: [Self; 2] = [Self::Fill, Self::Device];

    /// The name a saved tab and the lens control know it by.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Fill => "fill",
            Self::Device => "device",
        }
    }

    /// The lens a saved tab names. Anything else is `Fill`, as a tab with
    /// no lens kept is.
    #[must_use]
    pub fn from_key(key: &str) -> Self {
        match key {
            "device" => Self::Device,
            _ => Self::Fill,
        }
    }
}

/// One item tab as its canvas's file keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedTab {
    /// The page or Document the tab shows.
    pub item: EntityId,
    /// How the tab looks at it.
    pub lens: Lens,
}

/// The item tabs of one canvas.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Tabs {
    /// The items that have a tab, in the order they were opened. An item
    /// gets one when it is first shown alone and keeps it until it is
    /// closed.
    pub(crate) order: Vec<EntityId>,
    lenses: HashMap<EntityId, Lens>,
}

impl Tabs {
    /// The tabs a file kept, less those whose item `document` no longer
    /// has, or has as something that cannot be shown alone.
    pub(crate) fn from_saved(saved: Vec<SavedTab>, document: &Document) -> Self {
        let mut tabs = Self::default();
        for tab in saved {
            if tabs.open(&tab.item) {
                tabs.lenses.insert(tab.item, tab.lens);
            }
        }
        tabs.settle(document);
        tabs
    }

    /// The tabs as a save of `document` writes them, in order. A tab whose
    /// item is gone is left out.
    pub(crate) fn saved(&self, document: &Document) -> Vec<SavedTab> {
        (self.order.iter())
            .filter(|id| document.entity(id).is_some_and(can_show))
            .map(|id| SavedTab {
                item: id.clone(),
                lens: self.lens(id),
            })
            .collect()
    }

    /// The lens of `item`'s tab.
    pub(crate) fn lens(&self, item: &EntityId) -> Lens {
        self.lenses.get(item).copied().unwrap_or_default()
    }

    pub(crate) fn set_lens(&mut self, item: &EntityId, lens: Lens) {
        self.lenses.insert(item.clone(), lens);
    }

    /// Gives `item` a tab after the last one, unless it has one. Says
    /// whether it got one.
    pub(crate) fn open(&mut self, item: &EntityId) -> bool {
        let new = !self.order.contains(item);
        if new {
            self.order.push(item.clone());
        }
        new
    }

    /// Takes the tab of `item` away with its lens, and says
    /// where in the order it was.
    pub(crate) fn close(&mut self, item: &EntityId) -> Option<usize> {
        let at = self.order.iter().position(|id| id == item)?;
        self.order.remove(at);
        self.lenses.remove(item);
        Some(at)
    }

    /// Drops the tabs of items that are gone, with their lenses.
    pub(crate) fn settle(&mut self, document: &Document) {
        let stands = |id: &EntityId| document.entity(id).is_some_and(can_show);
        if self.order.iter().all(stands) {
            return;
        }
        self.order.retain(stands);
        let order = &self.order;
        self.lenses.retain(|id, _| order.contains(id));
    }
}
