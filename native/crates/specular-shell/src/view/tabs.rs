//! The tab row: the first row of the chrome. It is the window's title bar,
//! with the traffic lights in its left padding.

use std::cell::Cell;

use gpui_kit::component::h_flex;
use gpui_kit::{
    ClickEvent, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, px,
};

use crate::theme;

thread_local! {
    /// Whether a press on the bare strip is held and has not moved yet.
    static DRAG_ARMED: Cell<bool> = const { Cell::new(false) };
}

/// The room the traffic lights take, `toolbarPaddingLeft` on macOS.
const TRAFFIC_LIGHTS: f32 = 86.0;

/// A stretch of the row with nothing on it. The row is taller than the
/// title bar macOS drags and zooms by itself: here a press that moves drags
/// the window, and a double click does what the system setting says a
/// title bar's does.
fn bare_strip() -> impl IntoElement {
    h_flex()
        .id("tabs-bare")
        .flex_1()
        .min_w_0()
        .h_full()
        .on_mouse_down(MouseButton::Left, |_, _, _| DRAG_ARMED.set(true))
        .on_mouse_up(MouseButton::Left, |_, _, _| DRAG_ARMED.set(false))
        .on_mouse_down_out(|_, _, _| DRAG_ARMED.set(false))
        .on_mouse_move(|_, window, _| {
            if DRAG_ARMED.replace(false) {
                window.start_window_move();
            }
        })
        .on_click(|event: &ClickEvent, window, _| {
            if event.click_count() == 2 {
                window.titlebar_double_click();
            }
        })
}

/// The tab row. Tabs go between the traffic lights and the bare strip,
/// which takes whatever room they leave.
pub(super) fn tabs() -> impl IntoElement {
    h_flex()
        .h(px(theme::TAB_ROW))
        .flex_shrink_0()
        .pl(px(TRAFFIC_LIGHTS))
        .items_center()
        .child(bare_strip())
}
