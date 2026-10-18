//! How an item that is picked out is marked: the one answer the outline
//! and the resize handles both read, so what is drawn and what can be
//! grabbed agree.

use specular_doc::EntityId;

use crate::app::is_page;
use crate::{App, Lens};

/// The mark of an item that is selected, hovered or about to be taken by a
/// marquee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mark {
    /// The selection outline. Alone on the canvas it carries the resize
    /// handles.
    Outline,
    /// The entered page (ADR 0022), drawn as it is unselected: the page
    /// has the keys and the pointer, and nothing of the canvas's lies over
    /// its edge.
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
                let page = self.document.entity(id).is_some_and(is_page);
                (page && self.lens() == Some(Lens::Device)).then_some(Mark::Outline)
            }
        }
    }
}
