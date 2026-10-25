//! Where the Kit window draws over the canvas, so a headless run can turn
//! down a position that would land on the chrome in the window.
//!
//! A headless run lays the viewport out as the window does
//! (`Event::ChromeLayout`) but draws no panels, so a press under the tab row
//! would reach the canvas there and the tab row in the window. Failing it
//! headless keeps one script meaning one thing in both runners.

use glam::Vec2;
use specular_interact::panel::{DOCK_ROW, TAB_ROW};
use specular_interact::{App, ToolsPlace, dock, tool_options, toolbar};

/// How far right of the docked tools the armed tool's options reach, with
/// room to spare: two columns of buttons and the panel's padding.
const TOOL_OPTIONS_REACH: f32 = 72.0;

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
    // The armed tool's options stand in a column beside its docked button.
    let bar = toolbar(app);
    let docked = bar.tools == ToolsPlace::Docked;
    if docked && tool_options(app).is_some() && at.x < app.covered_left() + TOOL_OPTIONS_REACH {
        return Some("the armed tool's options");
    }
    if at.x >= viewport.x {
        return Some("the right panel");
    }
    // The dock's bar lies over the canvas's top while it holds something;
    // in a tab that shows its item the viewport already starts under it.
    let dock_shown = (tool_options(app).is_none() && dock(app).is_some()) || !bar.view.is_empty();
    if dock_shown && app.lens().is_none() && at.y < app.covered_top() + DOCK_ROW {
        return Some("the dock");
    }
    None
}
