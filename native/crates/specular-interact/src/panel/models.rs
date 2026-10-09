//! The two models themselves: the toolbar, and a set of controls. Their
//! controls are in [`model`](super::model).

use specular_doc::Color;

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

/// A button of the toolbar that shows and hides a side panel.
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarButton {
    /// Its name.
    pub id: ControlId,
    /// What it is called: it says what pressing it does.
    pub label: Label,
    /// The glyph.
    pub icon: Icon,
    /// Whether the panel is shown.
    pub open: bool,
    /// What pressing it does.
    pub action: Action,
}

/// The toolbar's theme button, which steps the theme through system, light
/// and dark.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeButton {
    /// Its name.
    pub id: ControlId,
    /// What it is called: the choice now in force.
    pub label: Label,
    /// The glyph of that choice.
    pub icon: Icon,
    /// What pressing it does: choose the next one.
    pub action: Action,
}

/// The toolbar as it is now.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolbarModel {
    /// The button for the right panel, at the right edge. Only a shell that
    /// has a right panel gets one.
    pub chat: Option<SidebarButton>,
    /// The theme button, just before the zoom readout.
    pub theme: ThemeButton,
    /// The blocks, left to right.
    pub sections: Vec<ToolbarSection>,
}

impl ToolbarModel {
    /// Every control and option with its action.
    pub fn entries(&self) -> Entries<'_> {
        let mut out = vec![(self.theme.id.clone(), Some(&self.theme.action))];
        out.extend((self.chat.iter()).map(|button| (button.id.clone(), Some(&button.action))));
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

/// A set of controls shown together: the dock's row for the tool in hand
/// or the selection, or the one list of choices that is a context menu.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlsModel {
    /// Its controls in order, with a separator between groups.
    pub controls: Vec<Control>,
}

impl ControlsModel {
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
