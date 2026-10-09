//! The three rows of the chrome, top to bottom: the tab row, the tool row
//! and the dock. This is the one place their heights are stacked.

use super::metrics::{DOCK_ROW, TAB_ROW, TOOL_ROW};
use super::node::PanelRect;

/// The title bar strip across a viewport `width` wide.
pub(super) fn tabs(width: f32) -> PanelRect {
    PanelRect::new(0.0, 0.0, width, TAB_ROW)
}

/// The row of tool buttons, under the tabs.
pub(super) fn tools(width: f32) -> PanelRect {
    PanelRect::new(0.0, tabs(width).bottom(), width, TOOL_ROW)
}

/// The dock, under the tools.
pub(super) fn dock(width: f32) -> PanelRect {
    PanelRect::new(0.0, tools(width).bottom(), width, DOCK_ROW)
}
