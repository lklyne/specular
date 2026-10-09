//! The tab row: the first row of the chrome. It is the window's title bar,
//! with the traffic lights in its left padding, then a tab for the canvas
//! and one for each page and Document.

use std::cell::Cell;

use gpui_kit::component::h_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    ClickEvent, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, div, px,
};
use specular_interact::{ViewStrip, ViewTab};

use super::controls::element_id;
use super::glyphs::{glyph, ink};
use super::named::mark;
use super::run;
use crate::theme;

thread_local! {
    /// Whether a press on the bare strip is held and has not moved yet.
    static DRAG_ARMED: Cell<bool> = const { Cell::new(false) };
}

/// The room the traffic lights take, `toolbarPaddingLeft` on macOS.
const TRAFFIC_LIGHTS: f32 = 86.0;
/// A tab: its width at rest, the least it shrinks to and its height.
const TAB: (f32, f32, f32) = (180.0, 28.0, 28.0);
/// How much of the row's far end stays bare, to drag the window by.
const BARE: f32 = 80.0;
const TAB_GLYPH: f32 = 14.0;

/// A stretch of the row with nothing on it. The row is taller than the
/// title bar macOS drags and zooms by itself: here a press that moves drags
/// the window, and a double click does what the system setting says a
/// title bar's does.
fn bare_strip() -> impl IntoElement {
    h_flex()
        .id("tabs-bare")
        .flex_1()
        .flex_shrink_0()
        .min_w(px(BARE))
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

/// One tab: its glyph and its label, cut short when the tab is narrow. The
/// one showing is filled.
fn tab(model: &ViewTab) -> impl IntoElement {
    let action = model.action.clone();
    let text = if model.active {
        theme::toolbar_text_strong()
    } else {
        theme::toolbar_text()
    };
    h_flex()
        .id(element_id(&model.id))
        .relative()
        .w(px(TAB.0))
        .min_w(px(TAB.1))
        .h(px(TAB.2))
        .flex_shrink(1.0)
        .px(px(7.0))
        .gap_1()
        .items_center()
        .overflow_hidden()
        .rounded(px(6.0))
        .cursor_pointer()
        .text_color(theme::solid(text))
        .when(model.active, |this| {
            this.bg(theme::solid(theme::tool_fill()))
        })
        .when(!model.active, |this| {
            this.hover(|this| this.bg(theme::solid(theme::tool_fill())))
        })
        .child(glyph(model.icon, ink(text), None, false, TAB_GLYPH))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(SharedString::from(model.label.to_string())),
        )
        .child(mark(&model.id))
        .on_click(move |_, window, cx| run(&action, window, cx))
}

/// The tab row of `model`. The tabs share what the traffic lights and the
/// bare strip leave, shrinking together; past their least width the row
/// cuts them off, so the strip is always there to drag the window by.
pub(super) fn tabs(model: &ViewStrip) -> impl IntoElement {
    h_flex()
        .h(px(theme::TAB_ROW))
        .flex_shrink_0()
        .pl(px(TRAFFIC_LIGHTS))
        .items_center()
        .child(
            h_flex()
                .min_w_0()
                .flex_shrink(1.0)
                .gap_1()
                .items_center()
                .overflow_hidden()
                .children(model.tabs.iter().map(tab)),
        )
        .child(bare_strip())
}
