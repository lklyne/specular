//! The toolbar: Kit buttons from [`ToolbarModel`], the zoom readout, and
//! the window's title. The strip is the title bar too: the traffic lights
//! sit in its left padding, as in the Electron app.

use std::cell::Cell;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{IconName, Selectable as _, Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, ClickEvent, Div, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    SharedString, Stateful, StatefulInteractiveElement as _, Styled as _, div, px,
};
use specular_interact::{
    PaintRole, Palette, SidebarButton, ThemeButton, ToolButton, ToolbarModel, ToolbarSection,
};

use super::controls::hint;
use super::dropdown::dropdown;
use super::glyphs::{self, glyph, ink};
use super::named::mark;
use super::run;
use crate::menus::Preferences;
use crate::theme;

thread_local! {
    /// Whether a press on the bare strip is held and has not moved yet.
    static DRAG_ARMED: Cell<bool> = const { Cell::new(false) };
}

/// The room the traffic lights take, `toolbarPaddingLeft` on macOS.
const TRAFFIC_LIGHTS: f32 = 86.0;
const TOOL_GLYPH: f32 = 20.0;
const PANEL_GLYPH: f32 = 14.0;

/// The surface a tool glyph's colour is resolved for: the pens show their
/// ink, the sticky and the shape their fill.
const fn tint_style(icon: specular_interact::Icon) -> (Palette, PaintRole) {
    match icon {
        specular_interact::Icon::DrawPenTool | specular_interact::Icon::DrawHighlightTool => {
            (Palette::Vivid, PaintRole::Ink)
        }
        _ => (Palette::Soft, PaintRole::Fill),
    }
}

fn tool(model: &ToolButton) -> impl IntoElement {
    let action = model.action.clone();
    let (palette, role) = tint_style(model.icon);
    let tint = (model.tint.as_ref()).map(|color| glyphs::resolved(color, palette, role));
    let current = ink(if model.active {
        theme::toolbar_text_strong()
    } else {
        theme::toolbar_text()
    });
    Button::new(SharedString::from(model.id.as_str().to_owned()))
        .ghost()
        .w(px(32.0))
        .h(px(28.0))
        // The Kit's own padding would leave a 20 px glyph 16 px of room.
        .px_0()
        .rounded(px(6.0))
        .selected(model.active)
        .when(model.active, |this| {
            this.bg(theme::solid(theme::tool_fill()))
        })
        .tooltip(hint(&model.label, model.chord))
        .child(glyph(model.icon, current, tint, model.active, TOOL_GLYPH))
        .child(mark(&model.id))
        .on_click(move |_, window, cx| run(&action, window, cx))
}

/// The button that shows or hides a side panel: its glyph is faint while
/// the panel is hidden.
fn panel_toggle(model: &SidebarButton) -> impl IntoElement {
    let action = model.action.clone();
    Button::new(SharedString::from(model.id.as_str().to_owned()))
        .ghost()
        .small()
        .rounded(px(8.0))
        .tooltip(SharedString::from(model.label.to_string()))
        .child(
            div()
                .when(!model.open, |this| this.opacity(0.6))
                .child(glyph(
                    model.icon,
                    ink(theme::toolbar_text()),
                    None,
                    false,
                    PANEL_GLYPH,
                )),
        )
        .child(mark(&model.id))
        .on_click(move |_, window, cx| run(&action, window, cx))
}

/// The button that shows and hides the sidebar, beside the traffic lights.
fn sidebar_button(model: &SidebarButton) -> impl IntoElement {
    let action = model.action.clone();
    let current = ink(if model.open {
        theme::toolbar_text_strong()
    } else {
        theme::toolbar_text()
    });
    Button::new(SharedString::from(model.id.as_str().to_owned()))
        .ghost()
        .w(px(32.0))
        .h(px(28.0))
        // The Kit's own padding would leave a 20 px glyph 16 px of room.
        .px_0()
        .rounded(px(6.0))
        .tooltip(hint(&model.label, None))
        .child(glyph(model.icon, current, None, false, 16.0))
        .child(mark(&model.id))
        .on_click(move |_, window, cx| run(&action, window, cx))
}

/// The button that moves the theme on.
fn theme_button(model: &ThemeButton) -> impl IntoElement {
    let action = model.action.clone();
    Button::new(SharedString::from(model.id.as_str().to_owned()))
        .ghost()
        .w(px(32.0))
        .h(px(28.0))
        // The Kit's own padding would leave a 20 px glyph 16 px of room.
        .px_0()
        .rounded(px(6.0))
        .tooltip(SharedString::from(model.label.to_string()))
        .child(glyph(
            model.icon,
            ink(theme::toolbar_text()),
            None,
            false,
            TOOL_GLYPH,
        ))
        .child(mark(&model.id))
        .on_click(move |_, window, cx| run(&action, window, cx))
}

fn divider() -> impl IntoElement {
    div()
        .mx_1()
        .w(px(1.0))
        .h(px(16.0))
        .bg(theme::tinted(theme::divider()))
}

/// A stretch of the strip with nothing on it. The strip is the title bar,
/// and taller than the one macOS drags and zooms by itself: here a press
/// that moves drags the window, and a double click does what the system
/// setting says a title bar's does.
fn bare_strip(id: &'static str) -> Stateful<Div> {
    h_flex()
        .id(id)
        .flex_1()
        .min_w_0()
        .h_full()
        .items_center()
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

/// The toolbar strip for `model`, with `title` beside the traffic lights.
pub(super) fn toolbar(model: &ToolbarModel, title: &str, _cx: &App) -> impl IntoElement {
    let mut cluster = h_flex().gap_1().items_center();
    for (index, section) in model.sections.iter().enumerate() {
        if index > 0 {
            cluster = cluster.child(divider());
        }
        cluster = match section {
            ToolbarSection::Tools(tools) => cluster.children(tools.iter().map(tool)),
            ToolbarSection::Zoom(zoom) => cluster
                .child(theme_button(&model.theme))
                .child(dropdown(zoom, true)),
        };
    }
    h_flex()
        .id("toolbar")
        .occlude()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(px(theme::TOOLBAR_HEIGHT))
        .pl(px(TRAFFIC_LIGHTS))
        .pr_4()
        .gap_1()
        .items_center()
        .bg(theme::solid(theme::toolbar()))
        .text_color(theme::solid(theme::toolbar_text()))
        .child(
            h_flex()
                .flex_1()
                .min_w_0()
                .h_full()
                .gap_2()
                .items_center()
                .child(sidebar_button(&model.sidebar))
                .child(
                    bare_strip("toolbar-title").child(
                        div()
                            .min_w_0()
                            .truncate()
                            .child(SharedString::from(title.to_owned())),
                    ),
                ),
        )
        .child(cluster)
        .child(
            h_flex()
                .flex_1()
                .gap_1()
                .h_full()
                .items_center()
                .child(bare_strip("toolbar-end"))
                .child(
                    Button::new("preferences")
                        .ghost()
                        .small()
                        .icon(IconName::Settings)
                        .tooltip("Settings  ⌘,")
                        .on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(Preferences), cx);
                        }),
                )
                .children(model.chat.as_ref().map(panel_toggle)),
        )
}
