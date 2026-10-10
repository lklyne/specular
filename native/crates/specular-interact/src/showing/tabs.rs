//! What a canvas's item tabs keep for the session: which items have one,
//! in what order, and for each its lens.

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
    /// The lens of `item`'s tab.
    pub(crate) fn lens(&self, item: &EntityId) -> Lens {
        self.lenses.get(item).copied().unwrap_or_default()
    }

    pub(crate) fn set_lens(&mut self, item: &EntityId, lens: Lens) {
        self.lenses.insert(item.clone(), lens);
    }

    /// Gives `item` a tab after the last one, unless it has one.
    pub(crate) fn open(&mut self, item: &EntityId) {
        if !self.order.contains(item) {
            self.order.push(item.clone());
        }
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
