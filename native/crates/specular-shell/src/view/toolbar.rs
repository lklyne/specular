//! The toolbar: Kit buttons from [`ToolbarModel`], the zoom readout, and
//! the window's title. The strip is the title bar too: the traffic lights
//! sit in its left padding, as in the Electron app.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{IconName, Selectable as _, Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, SharedString, Styled as _, div,
    px,
};
use specular_interact::{
    PaintRole, Palette, SidebarButton, ToolButton, ToolbarModel, ToolbarSection,
};

use super::controls::hint;
use super::dropdown::dropdown;
use super::glyphs::{self, glyph, ink};
use super::run;
use crate::menus::Preferences;
use crate::theme;

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
        theme::TOOLBAR_TEXT_STRONG
    } else {
        theme::TOOLBAR_TEXT
    });
    Button::new(SharedString::from(model.id.as_str().to_owned()))
        .ghost()
        .w(px(32.0))
        .h(px(28.0))
        // The Kit's own padding would leave a 20 px glyph 16 px of room.
        .px_0()
        .rounded(px(6.0))
        .selected(model.active)
        .when(model.active, |this| this.bg(theme::solid(theme::TOOL_FILL)))
        .tooltip(hint(&model.label, model.chord))
        .child(glyph(model.icon, current, tint, model.active, TOOL_GLYPH))
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
                    ink(theme::TOOLBAR_TEXT),
                    None,
                    false,
                    PANEL_GLYPH,
                )),
        )
        .on_click(move |_, window, cx| run(&action, window, cx))
}

/// The button that shows and hides the sidebar, beside the traffic lights.
fn sidebar_button(model: &SidebarButton) -> impl IntoElement {
    let action = model.action.clone();
    let current = ink(if model.open {
        theme::TOOLBAR_TEXT_STRONG
    } else {
        theme::TOOLBAR_TEXT
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
        .on_click(move |_, window, cx| run(&action, window, cx))
}

fn divider() -> impl IntoElement {
    div()
        .mx_1()
        .w(px(1.0))
        .h(px(16.0))
        .bg(theme::tinted(theme::DIVIDER))
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
            ToolbarSection::Zoom(zoom) => cluster.child(dropdown(zoom, true)),
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
        .bg(theme::solid(theme::TOOLBAR))
        .text_color(theme::solid(theme::TOOLBAR_TEXT))
        .child(
            h_flex()
                .flex_1()
                .min_w_0()
                .gap_2()
                .items_center()
                .child(sidebar_button(&model.sidebar))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .child(SharedString::from(title.to_owned())),
                ),
        )
        .child(cluster)
        .child(
            h_flex()
                .flex_1()
                .gap_1()
                .justify_end()
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
