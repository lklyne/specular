//! Where the Kit window draws over the canvas, so a headless run can turn
//! down a position that would land on the chrome in the window.
//!
//! A headless run lays the viewport out as the window does
//! (`Event::ChromeLayout`) but draws no panels, so a press under the tab row
//! would reach the canvas there and the tab row in the window. Failing it
//! headless keeps one script meaning one thing in both runners.

use glam::Vec2;
use specular_interact::chrome::{float_left_column, float_left_span};
use specular_interact::panel::{DOCK_ROW, TAB_ROW};
use specular_interact::{
    App, ToolbarModel, ToolbarSection, ToolsPlace, dock, tool_options, toolbar,
};

/// How far right of an upright ribbon the armed tool's options reach, with
/// room to spare: two columns of buttons and the panel's padding.
const TOOL_OPTIONS_REACH: f32 = 72.0;

/// Where `at`, in the viewport's points, is in the Kit window when the
/// canvas is not there: under which chrome, or outside the window.
pub(crate) fn kit_chrome_at(app: &App, at: Vec2) -> Option<&'static str> {
    let viewport = app.session().viewport;
    if at.y >= viewport.y || at.x < 0.0 {
        return Some("outside the window");
    }
    let regions = app.regions();
    let insets = app.insets();
    if regions.tabs && at.y < TAB_ROW {
        return Some("under the tab row");
    }
    if at.x < insets.left {
        return Some(if app.session().sidebar.shown() && regions.sidebar {
            "under the sidebar or the docked tools"
        } else {
            "under the docked tools"
        });
    }
    // The armed tool's options stand in a column beside its button, in a
    // docked ribbon or one floating down the canvas's left edge.
    let bar = toolbar(app);
    let ribbon_right = match bar.tools {
        _ if !regions.toolbar => None,
        ToolsPlace::Docked => Some(insets.left),
        ToolsPlace::FloatLeft => {
            let column = float_left_column(insets.left);
            let span = floating_span(&bar, insets.top, viewport.y);
            if column.contains(&at.x) && span.contains(&at.y) {
                return Some("under the floating tools");
            }
            span.contains(&at.y).then_some(column.end)
        }
        ToolsPlace::FloatTop | ToolsPlace::FloatBottom => None,
    };
    if let Some(right) = ribbon_right
        && tool_options(app).is_some()
        && at.x < right + TOOL_OPTIONS_REACH
    {
        return Some("under the armed tool's options");
    }
    if at.x >= viewport.x {
        return Some("under the right panel");
    }
    // The dock's bar lies over the canvas's top while it holds something;
    // in a tab that shows its item the viewport already starts under it.
    let dock_shown = (tool_options(app).is_none() && dock(app).is_some()) || !bar.view.is_empty();
    if regions.dock && dock_shown && app.lens().is_none() && at.y < insets.top + DOCK_ROW {
        return Some("under the dock");
    }
    None
}

/// The span down the window of the ribbon floating down the canvas's left
/// edge, between `top` and `bottom`: its tools, a rule between groups, then
/// a rule, the zoom readout a mod puts there and the theme button.
fn floating_span(bar: &ToolbarModel, top: f32, bottom: f32) -> std::ops::Range<f32> {
    let groups: Vec<usize> = (bar.sections.iter())
        .filter_map(|section| match section {
            ToolbarSection::Tools(tools) => Some(tools.len()),
            ToolbarSection::Zoom(_) => None,
        })
        .collect();
    let items = groups.iter().sum::<usize>() + 2;
    let rules = groups.len();
    float_left_span(top, bottom, items, rules)
}
