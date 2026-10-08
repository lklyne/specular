//! The wheel over the sidebar scrolls its list.

use super::Surface;
use super::sidebar::scroll_range;
use crate::{App, WheelInput};

/// Scrolls the sidebar's list when the pointer is over the sidebar, which
/// takes the wheel whether or not there is anything to scroll to. Returns
/// whether it did, in which case the canvas sees none of it.
pub(crate) fn on_wheel(app: &mut App, input: &WheelInput) -> bool {
    if !app.session.panel.built_in || !app.session.sidebar.shown() {
        return false;
    }
    let Some(pointer) = app.session.pointer else {
        return false;
    };
    let over = super::hit(app, pointer)
        .is_some_and(|hit| matches!(hit.surface, Surface::Sidebar | Surface::SidebarList));
    if !over {
        return false;
    }
    let range = scroll_range(&crate::sidebar(app), app.session.viewport);
    // Positive `y` moves content down, which is scrolling back up.
    let panel = &mut app.session.panel;
    panel.sidebar_scroll = (panel.sidebar_scroll - input.delta.y).clamp(0.0, range);
    // Different rows are under the pointer now.
    let layout = super::layout(app);
    app.session.panel.hover = (layout.hit(pointer))
        .and_then(|hit| hit.control)
        .filter(|id| layout.node(id).is_some_and(|node| node.state.enabled));
    true
}
