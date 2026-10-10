//! The toolbar's places, all drawn from [`ToolbarModel`]: the tools and the
//! theme in a ribbon, docked beside the canvas or floating over it, with
//! the options of the tool in hand beside its button; the zoom readout and
//! the right panel's toggle at the end of the tab row; and the lens of the
//! tab showing at the end of the dock's bar.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{Selectable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    Styled as _, Window, div, px,
};
use specular_interact::{
    ControlId, ControlsModel, PaintRole, Palette, SidebarButton, TOOLS_DOCK_WIDTH, ThemeButton,
    ToolButton, ToolbarModel, ToolbarSection, ToolsPlace,
};

use super::controls::{column, control, hint};
use super::dropdown::dropdown;
use super::glyphs::{self, glyph, ink};
use super::named::mark;
use super::run;
use crate::theme;

/// The side of a tool's button, which is square.
const TOOL_BUTTON: f32 = 28.0;
const TOOL_GLYPH: f32 = 20.0;
const PANEL_GLYPH: f32 = 14.0;
/// From the canvas's edge to a floating ribbon, and from a ribbon's own
/// edge to its buttons.
const RIBBON_INSET: f32 = 8.0;
const RIBBON_PAD: f32 = 4.0;
/// From a tool's button to its options.
const OPTIONS_GAP: f32 = 10.0;

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

fn tool(model: &ToolButton, tip: bool) -> impl IntoElement {
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
        .size(px(TOOL_BUTTON))
        // The Kit's own padding would leave a 20 px glyph 12 px of room.
        .px_0()
        .rounded(px(6.0))
        .selected(model.active)
        .when(model.active, |this| {
            this.bg(theme::solid(theme::tool_fill()))
        })
        .child(glyph(model.icon, current, tint, model.active, TOOL_GLYPH))
        .when(tip, |this| {
            this.child(crate::tip::over(
                model.id.as_str(),
                hint(&model.label, model.chord),
            ))
        })
        .child(mark(&model.id))
        .on_click(move |_, window, cx| run(&action, window, cx))
}

/// The button that shows or hides a side panel: its glyph is faint while
/// the panel is hidden.
pub(super) fn panel_toggle(model: &SidebarButton) -> impl IntoElement {
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

/// What a docked button is called, shown right of the ribbon and level
/// with the button while the pointer is over `group`. A tooltip would open
/// under the pointer, over the tools below.
fn beside(group: &ControlId, text: SharedString) -> impl IntoElement {
    div()
        .absolute()
        .left(px(TOOL_BUTTON + OPTIONS_GAP))
        .top_0()
        .h(px(TOOL_BUTTON))
        .flex()
        .items_center()
        .invisible()
        .group_hover(SharedString::from(group.as_str().to_owned()), |this| {
            this.visible()
        })
        .child(
            div()
                .px_2()
                .py_1()
                .rounded(px(6.0))
                .bg(theme::solid(theme::popup()))
                .border_1()
                .border_color(theme::solid(theme::chrome_border()))
                .shadow_md()
                .whitespace_nowrap()
                .text_color(theme::solid(theme::text()))
                .child(text),
        )
}

/// The button that moves the theme on, the last of the ribbon. `tip` is
/// whether it has a tooltip of its own.
fn theme_button(model: &ThemeButton, tip: bool) -> impl IntoElement {
    let action = model.action.clone();
    Button::new(SharedString::from(model.id.as_str().to_owned()))
        .ghost()
        .size(px(TOOL_BUTTON))
        // The Kit's own padding would leave a 20 px glyph 12 px of room.
        .px_0()
        .rounded(px(6.0))
        .when(tip, |this| {
            this.child(crate::tip::over(model.id.as_str(), model.label.to_string()))
        })
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

/// A rule between two groups of tools, across the way the ribbon runs.
fn rule(upright: bool) -> impl IntoElement {
    let (w, h) = if upright { (1.0, 16.0) } else { (16.0, 1.0) };
    div()
        .m_1()
        .w(px(w))
        .h(px(h))
        .flex_shrink_0()
        .bg(theme::tinted(theme::divider()))
}

/// The surface of something that floats over the canvas.
fn floats(id: &'static str) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .occlude()
        .flex()
        .p(px(RIBBON_PAD))
        .gap_1()
        .items_center()
        .rounded(px(10.0))
        .bg(theme::solid(theme::toolbar()))
        .border_1()
        .border_color(theme::solid(theme::chrome_border()))
        .shadow_md()
        .text_color(theme::solid(theme::toolbar_text()))
}

/// The options of the tool in hand, beside its button: a column right of a
/// docked ribbon, a row under one floating at the top and over one at the
/// bottom.
fn options(
    model: &ControlsModel,
    place: ToolsPlace,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let panel = floats("tool-options").absolute();
    match place {
        ToolsPlace::Docked => panel
            .flex_col()
            .top(px(-RIBBON_PAD))
            .left(px(TOOL_BUTTON + OPTIONS_GAP))
            .children(column(&model.controls, window, cx)),
        ToolsPlace::FloatTop | ToolsPlace::FloatBottom => {
            let controls: Vec<_> = (model.controls.iter())
                .map(|model| control(model, window, cx))
                .collect();
            panel
                .left(px(-RIBBON_PAD))
                .when(place == ToolsPlace::FloatTop, |this| {
                    this.top(px(TOOL_BUTTON + OPTIONS_GAP))
                })
                .when(place == ToolsPlace::FloatBottom, |this| {
                    this.bottom(px(TOOL_BUTTON + OPTIONS_GAP))
                })
                .children(controls)
        }
    }
}

/// The tool buttons of `model`, a rule between groups, then the theme's.
/// The tool in hand has `shown` beside it.
fn tools(
    model: &ToolbarModel,
    shown: Option<&ControlsModel>,
    window: &mut Window,
    cx: &mut App,
) -> Vec<AnyElement> {
    let upright = model.tools != ToolsPlace::Docked;
    let groups = model.sections.iter().filter_map(|section| match section {
        ToolbarSection::Tools(tools) => Some(tools),
        ToolbarSection::Zoom(_) => None,
    });
    let mut out = Vec::new();
    for (index, group) in groups.enumerate() {
        if index > 0 {
            out.push(rule(upright).into_any_element());
        }
        let docked = model.tools == ToolsPlace::Docked;
        for button in group {
            let open = shown.filter(|_| button.active);
            // Docked, a button is named beside the ribbon, where its
            // options are when it has them open.
            let name = (docked && open.is_none())
                .then(|| beside(&button.id, hint(&button.label, button.chord)));
            out.push(
                div()
                    .group(SharedString::from(button.id.as_str().to_owned()))
                    .relative()
                    .child(tool(button, !docked))
                    .children(name)
                    .children(open.map(|shown| options(shown, model.tools, window, cx)))
                    .into_any_element(),
            );
        }
    }
    out
}

/// The tools docked: a strip down the canvas's left edge, `left` from the
/// window's, which the app counts as covered. The theme is at its foot.
pub(super) fn docked(
    model: &ToolbarModel,
    shown: Option<&ControlsModel>,
    left: f32,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    v_flex()
        .id("ribbon")
        .occlude()
        .absolute()
        .left(px(left))
        .top(px(theme::CHROME_HEIGHT))
        .bottom_0()
        .w(px(TOOLS_DOCK_WIDTH))
        .py(px(RIBBON_PAD))
        .gap_1()
        .items_center()
        .bg(theme::solid(theme::toolbar()))
        .border_r_1()
        .border_color(theme::solid(theme::toolbar_border()))
        .text_color(theme::solid(theme::toolbar_text()))
        .children(tools(model, shown, window, cx))
        .child(div().flex_1())
        .child(
            div()
                .group(SharedString::from(model.theme.id.as_str().to_owned()))
                .relative()
                .child(theme_button(&model.theme, false))
                .child(beside(
                    &model.theme.id,
                    model.theme.label.to_string().into(),
                )),
        )
}

/// The tools floating: a ribbon centred over the top or the bottom of what
/// the canvas is seen through, which starts `left` from the slot's edge and
/// `top` from its top. It covers nothing the app counts.
pub(super) fn floating(
    model: &ToolbarModel,
    shown: Option<&ControlsModel>,
    (left, top): (f32, f32),
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let ribbon = floats("ribbon")
        .children(tools(model, shown, window, cx))
        .child(rule(true))
        .child(theme_button(&model.theme, true));
    h_flex()
        .absolute()
        .left(px(left))
        .right_0()
        .when(model.tools == ToolsPlace::FloatTop, |this| {
            this.top(px(top + RIBBON_INSET))
        })
        .when(model.tools == ToolsPlace::FloatBottom, |this| {
            this.bottom(px(RIBBON_INSET))
        })
        .justify_center()
        .child(ribbon)
}

/// The zoom readout, for the end of the tab row.
pub(super) fn zoom(model: &ToolbarModel) -> Option<AnyElement> {
    model.sections.iter().find_map(|section| match section {
        ToolbarSection::Zoom(zoom) => Some(dropdown(zoom)),
        ToolbarSection::Tools(_) => None,
    })
}

/// The lens and the eye of the tab showing, for the end of the dock's bar.
pub(super) fn lens(model: &ToolbarModel, window: &mut Window, cx: &mut App) -> Vec<AnyElement> {
    (model.view.iter())
        .map(|model| control(model, window, cx))
        .collect()
}
