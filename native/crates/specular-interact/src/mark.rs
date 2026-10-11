//! How an item that is picked out is marked: the one answer the outline,
//! the resize handles and a page's glow all read, so what is drawn and what
//! can be grabbed agree.

use specular_doc::EntityId;

use crate::app::page_of;
use crate::{App, Lens};

/// The mark of an item that is selected, hovered or about to be taken by a
/// marquee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mark {
    /// The selection outline. Alone on the canvas it carries the resize
    /// handles.
    Outline,
    /// The glow of the entered page (ADR 0022), in place of the outline:
    /// the page has the keys and the pointer, and nothing of the canvas's
    /// lies over its edge.
    Entered,
}

impl App {
    /// How `id` is marked when it is picked out, or `None` when it has no
    /// mark. The item shown in a tab is the whole view, so it has none
    /// while it fills the view or is entered. A page shown in Device that
    /// was left by Escape is outlined: it is waiting for the click that
    /// enters it.
    pub fn mark(&self, id: &EntityId) -> Option<Mark> {
        let shown = self.shown_item() == Some(id);
        let entered = self.session.focus.page() == Some(id);
        match (shown, entered) {
            (false, false) => Some(Mark::Outline),
            (false, true) => Some(Mark::Entered),
            (true, true) => None,
            (true, false) => {
                let page = self.document.entity(id).and_then(page_of).is_some();
                (page && self.lens() == Some(Lens::Device)).then_some(Mark::Outline)
            }
        }
    }

    /// Whether the dock already says what the page `id` is: it is the whole
    /// selection or the item shown, so its address field is in the dock and
    /// the title above it would say the same thing twice.
    pub fn names_in_dock(&self, id: &EntityId) -> bool {
        self.shown_item() == Some(id) || self.session.selection.single_entity() == Some(id)
    }
}
