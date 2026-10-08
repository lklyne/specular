//! What the sidebar remembers outside the document: whether it is shown,
//! which sections are folded and which rows are open.
//!
//! This is session state, not the built-in renderer's: a renderer drawn by
//! a UI library needs the same answers, and the model reports them. What
//! only the built-in renderer needs, the scroll offset and the hover, is in
//! [`PanelUi`](crate::PanelUi).

use std::collections::HashSet;

use specular_doc::EntityId;

/// The width the sidebar covers when it is shown: `LEFT_SIDEBAR_WIDTH` in
/// `runtime-constants.ts`.
pub const SIDEBAR_WIDTH: f32 = 256.0;

/// The three folds of the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SidebarSection {
    /// The list of canvases.
    Canvases,
    /// Everything on the canvas that is not a page.
    Notes,
    /// The pages of the canvas and what is hooked to them.
    Pages,
}

/// A change to the sidebar's own state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarAction {
    /// Show it, or hide it.
    Toggle,
    /// Fold a section, or unfold it.
    Section(SidebarSection),
    /// Open a row that has rows inside it, or close it.
    Row {
        /// The section the row is in. A group with notes and pages has a row
        /// in each, and they open apart.
        section: SidebarSection,
        /// The group or page.
        entity: EntityId,
    },
}

/// The sidebar's state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SidebarView {
    shown: bool,
    folded: HashSet<SidebarSection>,
    /// Rows whose state is the opposite of what their kind starts as.
    flipped: HashSet<(SidebarSection, EntityId)>,
}

impl SidebarView {
    /// Whether the sidebar is shown. It starts hidden.
    pub fn shown(&self) -> bool {
        self.shown
    }

    /// How much of the canvas viewport's left edge the sidebar covers.
    pub fn covered_width(&self) -> f32 {
        if self.shown { SIDEBAR_WIDTH } else { 0.0 }
    }

    /// Whether `section` is folded away.
    pub fn is_folded(&self, section: SidebarSection) -> bool {
        self.folded.contains(&section)
    }

    /// Whether the row for `entity` in `section` is open. A page starts
    /// open and a group closed, which `starts_open` says.
    pub fn is_open(&self, section: SidebarSection, entity: &EntityId, starts_open: bool) -> bool {
        starts_open != self.flipped.contains(&(section, entity.clone()))
    }

    /// Applies `action`.
    pub(crate) fn apply(&mut self, action: SidebarAction) {
        match action {
            SidebarAction::Toggle => self.shown = !self.shown,
            SidebarAction::Section(section) => {
                if !self.folded.remove(&section) {
                    self.folded.insert(section);
                }
            }
            SidebarAction::Row { section, entity } => {
                let key = (section, entity);
                if !self.flipped.remove(&key) {
                    self.flipped.insert(key);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_flips_from_how_its_kind_starts() {
        let mut view = SidebarView::default();
        let page = EntityId::from("p");
        assert!(view.is_open(SidebarSection::Pages, &page, true));
        view.apply(SidebarAction::Row {
            section: SidebarSection::Pages,
            entity: page.clone(),
        });
        assert!(!view.is_open(SidebarSection::Pages, &page, true));
        assert!(
            view.is_open(SidebarSection::Notes, &page, true),
            "the other section's row is its own"
        );
    }

    #[test]
    fn covering_follows_whether_it_is_shown() {
        let mut view = SidebarView::default();
        assert_eq!(view.covered_width(), 0.0);
        view.apply(SidebarAction::Toggle);
        assert_eq!(view.covered_width(), SIDEBAR_WIDTH);
    }
}
