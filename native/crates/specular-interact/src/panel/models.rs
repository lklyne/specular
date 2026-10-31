//! The two models themselves: the toolbar, and a set of controls. Their
//! controls are in [`model`](super::model).

use specular_doc::Color;

use super::model::{Button, Control, Dropdown, Label};
use super::{ControlId, Icon};
use crate::{Action, Chord, Tool, ToolsPlace};

/// A tool button of the toolbar.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolButton {
    /// Its name.
    pub id: ControlId,
    /// The tool it arms.
    pub(crate) tool: Tool,
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

/// The Share button at the right end of the tab row, and the popover it
/// opens.
#[derive(Debug, Clone, PartialEq)]
pub struct ShareButton {
    /// Its name.
    pub id: ControlId,
    /// Who the link lets in: only you, or anyone with it, to view, comment
    /// or edit.
    pub access: Dropdown,
    /// Copies the link, sharing the canvas first when it is not shared.
    pub copy: Button,
    /// Why the last share failed, under the button.
    pub note: Option<Label>,
}

/// The toolbar's theme button, which flips the theme between light and
/// dark.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeButton {
    /// Its name.
    pub id: ControlId,
    /// What it is called: what pressing it does.
    pub label: Label,
    /// The glyph of what is drawn now.
    pub icon: Icon,
    /// What pressing it does: draw the other, as a fixed choice or by
    /// following the system again.
    pub action: Action,
}

/// The toolbar as it is now.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolbarModel {
    /// The button for the right panel, at the right edge. Only a shell that
    /// has a right panel gets one.
    pub chat: Option<SidebarButton>,
    /// The Share button, just before the right panel's.
    pub share: ShareButton,
    /// The theme button, just before the zoom readout.
    pub theme: ThemeButton,
    /// The blocks, left to right.
    pub sections: Vec<ToolbarSection>,
    /// Where the tools' ribbon is.
    pub tools: ToolsPlace,
    /// How the tab showing looks at its item, at the right end: a toggle
    /// Minimize, then a toggle for each lens. On the Canvas tab, the
    /// button that opens the one page or Document selected in a tab, or
    /// nothing.
    pub view: Vec<Control>,
}

impl ToolbarModel {}

/// A set of controls shown together: the dock's row for the tool in hand
/// or the selection, or the one list of choices that is a context menu.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlsModel {
    /// Its controls in order, with a separator between groups.
    pub controls: Vec<Control>,
}

impl ControlsModel {}
