//! What a canvas's item tabs keep for the session: their order, and for
//! each one its lens and the camera of its Canvas lens.

use std::collections::HashMap;

use specular_core::Camera;
use specular_doc::{Document, EntityId};

use super::{can_show, listed};

/// How a tab looks at its item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Lens {
    /// The item fills the area under the chrome, as a browser tab or an
    /// editor does.
    #[default]
    Fill,
    /// The item at its stored size, fitted into the window.
    Device,
    /// The canvas around the item, with a camera of the tab's own.
    Canvas,
}

impl Lens {
    /// Every lens, in the order the control lists them.
    pub const ALL: [Self; 3] = [Self::Fill, Self::Device, Self::Canvas];
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Kept {
    lens: Lens,
    /// Where the Canvas lens was left. `None` until it has been looked
    /// through, when it starts fitted to the item.
    camera: Option<Camera>,
}

/// The item tabs of one canvas.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Tabs {
    /// The items that can be shown alone, in the order their tabs keep.
    pub(crate) order: Vec<EntityId>,
    kept: HashMap<EntityId, Kept>,
}

impl Tabs {
    /// The lens of `item`'s tab.
    pub(crate) fn lens(&self, item: &EntityId) -> Lens {
        self.kept.get(item).map_or_else(Lens::default, |it| it.lens)
    }

    pub(crate) fn set_lens(&mut self, item: &EntityId, lens: Lens) {
        self.kept.entry(item.clone()).or_default().lens = lens;
    }

    /// Where the Canvas lens of `item`'s tab was left.
    pub(crate) fn camera(&self, item: &EntityId) -> Option<Camera> {
        self.kept.get(item).and_then(|it| it.camera)
    }

    pub(crate) fn keep_camera(&mut self, item: &EntityId, camera: Camera) {
        self.kept.entry(item.clone()).or_default().camera = Some(camera);
    }

    /// Takes new items into the order and drops gone ones, with what was
    /// kept for them.
    pub(crate) fn settle(&mut self, document: &Document) {
        let count = document.entities().filter(|it| can_show(it)).count();
        let stands = count == self.order.len()
            && (self.order.iter()).all(|id| document.entity(id).is_some_and(can_show));
        if stands {
            return;
        }
        self.order = (listed(document, &self.order).into_iter())
            .map(|entity| entity.id.clone())
            .collect();
        let order = &self.order;
        self.kept.retain(|id, _| order.contains(id));
    }
}
