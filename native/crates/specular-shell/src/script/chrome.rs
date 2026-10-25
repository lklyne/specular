//! Where the Kit window draws over the canvas, so a headless run can turn
//! down a position that would land on the chrome in the window.
//!
//! A headless run lays the viewport out as the window does
//! (`Event::ChromeLayout`) but draws no panels, so a press under the tab row
//! would reach the canvas there and the tab row in the window. Failing it
//! headless keeps one script meaning one thing in both runners.

use glam::Vec2;
use specular_interact::panel::{DOCK_ROW, TAB_ROW};
use specular_interact::{App, dock, tool_options, toolbar};

/// What the Kit draws at `at`, in the viewport's points, or `None` when the
/// canvas is there.
pub(crate) fn kit_chrome_at(app: &App, at: Vec2) -> Option<&'static str> {
    let viewport = app.session().viewport;
    if at.y < TAB_ROW {
        return Some("the tab row");
    }
    if at.x < app.covered_left() {
        return Some(if app.session().sidebar.shown() {
            "the sidebar or the docked tools"
        } else {
            "the docked tools"
        });
    }
    if at.x >= viewport.x {
        return Some("the right panel");
    }
    // The dock's bar lies over the canvas's top while it holds something;
    // in a tab that shows its item the viewport already starts under it.
    let bar = toolbar(app);
    let docked = (tool_options(app).is_none() && dock(app).is_some()) || !bar.view.is_empty();
    if docked && app.lens().is_none() && at.y < app.covered_top() + DOCK_ROW {
        return Some("the dock");
    }
    None
}
