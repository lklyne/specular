//! The two models themselves: the toolbar, and a popup with where it
//! belongs. Their controls are in [`model`](super::model).

use glam::Vec2;
use specular_doc::{Color, Rect};

use super::model::{Control, Dropdown, Entries, Label};
use super::{ControlId, Icon};
use crate::{Action, Chord, Tool};

/// A tool button of the toolbar.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolButton {
    /// Its name.
    pub id: ControlId,
    /// The tool it arms.
    pub tool: Tool,
    /// What it is called.
    pub label: Label,
    /// The glyph.
    pub icon: Icon,
    /// What the glyph is painted in, for the tools whose glyph shows their
    /// current color.
    pub tint: Option<Color>,
    /// Whether the tool is the active one.
    pub active: bool,
    /// The key that arms the tool.
    pub chord: Option<Chord>,
    /// What pressing it does: arm the tool, or put it down when it is one
    /// that a second press puts down.
    pub action: Action,
}

/// One block of the toolbar. Blocks are set apart from each other.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolbarSection {
    /// Tool buttons.
    Tools(Vec<ToolButton>),
    /// The zoom readout, which opens the zoom levels.
    Zoom(Dropdown),
}

/// The button at the toolbar's left edge that shows and hides the sidebar.
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarButton {
    /// Its name.
    pub id: ControlId,
    /// What it is called: it says what pressing it does.
    pub label: Label,
    /// The glyph.
    pub icon: Icon,
    /// Whether the sidebar is shown.
    pub open: bool,
    /// What pressing it does.
    pub action: Action,
}

/// The toolbar as it is now.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolbarModel {
    /// The button for the sidebar, at the left edge.
    pub sidebar: SidebarButton,
    /// The blocks, left to right.
    pub sections: Vec<ToolbarSection>,
}

impl ToolbarModel {
    /// Every control and option with its action.
    pub fn entries(&self) -> Entries<'_> {
        let mut out = vec![(self.sidebar.id.clone(), Some(&self.sidebar.action))];
        for section in &self.sections {
            match section {
                ToolbarSection::Tools(tools) => {
                    out.extend(tools.iter().map(|t| (t.id.clone(), Some(&t.action))));
                }
                ToolbarSection::Zoom(dropdown) => dropdown.entries(&mut out),
            }
        }
        out
    }

    /// The action of the control or option named `id`.
    pub fn action(&self, id: &ControlId) -> Option<Action> {
        (self.entries().into_iter())
            .find(|(entry, _)| entry == id)
            .and_then(|(_, action)| action.cloned())
    }
}

/// Which side of what it points at a popup sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Over it.
    Above,
    /// Under it.
    Below,
}

/// How a popup lines up with what it points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// Centered on it.
    Center,
    /// Centered, and at least as wide as it.
    Stretch,
}

/// Where a popup belongs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopupAnchor {
    /// Under the toolbar, centered on it: the popup of the active tool.
    Toolbar {
        /// The space between the toolbar and the popup, in screen pixels.
        gap: f32,
    },
    /// Beside a region of the canvas.
    Canvas {
        /// The region, in canvas units: the item, the union of the
        /// selection, or the middle point of an edge as an empty rect.
        bounds: Rect,
        /// Which side of it.
        placement: Placement,
        /// How it lines up.
        align: Align,
        /// The space between the region and the popup, in screen pixels.
        gap: f32,
    },
    /// At a point of the viewport, in screen pixels: a context menu, which
    /// opens where the pointer was.
    Point(Vec2),
}

/// The popup of the tool in hand or of the selection.
#[derive(Debug, Clone, PartialEq)]
pub struct PopupModel {
    /// Where it belongs.
    pub anchor: PopupAnchor,
    /// Its controls in order, with a separator between groups.
    pub controls: Vec<Control>,
}

impl PopupModel {
    /// Every control and option with its action.
    pub fn entries(&self) -> Entries<'_> {
        let mut out = Vec::new();
        for control in &self.controls {
            control.entries(&mut out);
        }
        out
    }

    /// The action of the control or option named `id`.
    pub fn action(&self, id: &ControlId) -> Option<Action> {
        (self.entries().into_iter())
            .find(|(entry, _)| entry == id)
            .and_then(|(_, action)| action.cloned())
    }
}
