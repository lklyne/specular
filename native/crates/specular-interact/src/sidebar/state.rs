//! What the sidebar remembers outside the document: whether it is shown,
//! which sections are folded and which rows are open.
//!
//! This is session state, so whatever draws the sidebar reads it from the
//! model and keeps none of it.

use std::collections::HashSet;

use specular_doc::EntityId;

use crate::ProjectId;

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
#[derive(Debug, Clone, PartialEq)]
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
    /// Fold a project's canvases away, or show them.
    Project(ProjectId),
    /// Make it this wide, within its limits: its right edge dragged.
    Resize(f32),
}

/// The narrowest the sidebar is dragged to.
pub(crate) const SIDEBAR_MIN_WIDTH: f32 = 200.0;
/// The widest the sidebar is dragged to.
pub(crate) const SIDEBAR_MAX_WIDTH: f32 = 520.0;

/// The sidebar's state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SidebarView {
    shown: bool,
    folded: HashSet<SidebarSection>,
    /// Rows whose state is the opposite of what their kind starts as.
    flipped: HashSet<(SidebarSection, EntityId)>,
    folded_projects: HashSet<ProjectId>,
    /// The width it was dragged to, or none for the layout's.
    width: Option<f32>,
}

impl SidebarView {
    /// Whether the sidebar is shown. It starts hidden.
    pub fn shown(&self) -> bool {
        self.shown
    }

    /// The width it was dragged to, or `None` while it is the layout's.
    pub fn width(&self) -> Option<f32> {
        self.width
    }

    /// Whether `section` is folded away.
    pub(crate) fn is_folded(&self, section: SidebarSection) -> bool {
        self.folded.contains(&section)
    }

    /// Whether the project `id`'s canvases are folded away.
    pub(crate) fn is_project_folded(&self, id: &ProjectId) -> bool {
        self.folded_projects.contains(id)
    }

    /// Whether the row for `entity` in `section` is open. A page starts
    /// open and a group closed, which `starts_open` says.
    pub(crate) fn is_open(
        &self,
        section: SidebarSection,
        entity: &EntityId,
        starts_open: bool,
    ) -> bool {
        starts_open != self.flipped.contains(&(section, entity.clone()))
    }

    /// Applies `action`.
    pub(crate) fn apply(&mut self, action: SidebarAction) {
        match action {
            SidebarAction::Toggle => self.shown = !self.shown,
            SidebarAction::Resize(width) => {
                if width.is_finite() {
                    self.width = Some(width.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH));
                }
            }
            SidebarAction::Section(section) => {
                if !self.folded.remove(&section) {
                    self.folded.insert(section);
                }
            }
            SidebarAction::Project(id) => {
                if !self.folded_projects.remove(&id) {
                    self.folded_projects.insert(id);
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
