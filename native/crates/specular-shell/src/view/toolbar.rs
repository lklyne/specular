//! The toolbar: the second row of the chrome, with Kit buttons from
//! [`ToolbarModel`] and the zoom readout.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{IconName, Selectable as _, Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{App, IntoElement, ParentElement as _, SharedString, Styled as _, Window, div, px};
use specular_interact::{
    PaintRole, Palette, SidebarButton, ThemeButton, ToolButton, ToolbarModel, ToolbarSection,
};

use super::controls::{control, hint};
use super::dropdown::dropdown;
use super::glyphs::{self, glyph, ink};
use super::named::mark;
use super::run;
use crate::menus::Preferences;
use crate::theme;

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
        .child(glyph(model.icon, current, tint, model.active, TOOL_GLYPH))
        .child(crate::tip::over(
            model.id.as_str(),
            hint(&model.label, model.chord),
        ))
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
        .child(crate::tip::over(model.id.as_str(), model.label.to_string()))
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
        .child(crate::tip::over(model.id.as_str(), model.label.to_string()))
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

/// The toolbar's row for `model`: the tools centred, and at the far end the
/// lens and the eye of the tab showing, settings and the right panel's
/// toggle.
pub(super) fn toolbar(model: &ToolbarModel, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let view: Vec<_> = (model.view.iter())
        .map(|model| control(model, window, cx))
        .collect();
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
        .h(px(theme::TOOL_ROW))
        .flex_shrink_0()
        .px_4()
        .gap_1()
        .items_center()
        .text_color(theme::solid(theme::toolbar_text()))
        .child(div().flex_1())
        .child(cluster)
        .child(
            h_flex()
                .flex_1()
                .gap_1()
                .h_full()
                .items_center()
                .justify_end()
                .children(view)
                .child(
                    Button::new("preferences")
                        .ghost()
                        .small()
                        .icon(IconName::Settings)
                        .child(crate::tip::over("preferences", "Settings  ⌘,"))
                        .on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(Preferences), cx);
                        }),
                )
                .children(model.chat.as_ref().map(panel_toggle)),
        )
}
