//! The toolbar's places, all drawn from [`ToolbarModel`]: the tools and the
//! theme in a ribbon, docked beside the canvas or floating over it, with
//! the options of the tool in hand beside its button; the zoom readout and
//! the right panel's toggle at the end of the tab row; and the lens of the
//! tab showing at the end of the dock's bar.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{Selectable as _, Sizable as _, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    Styled as _, Window, div, px,
};
use specular_interact::chrome::{OPTIONS_GAP, RIBBON_PAD, TOOL_BUTTON};
use specular_interact::{
    ControlId, ControlsModel, PaintRole, Palette, SidebarButton, ThemeButton, ToolButton,
    ToolbarModel, ToolbarSection, ToolsPlace,
};

use super::controls::{column, control, hint};
use super::dropdown::{dropdown, dropdown_compact};
use super::glyphs::{self, glyph, ink};
use super::named::mark;
use super::raised::Raised;
use super::run;
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
        .rounded(crate::theme::corner(6.0))
        .selected(model.active)
        .when(model.active, |this| this.raised(6.0))
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
        .rounded(crate::theme::corner(8.0))
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
                .px(crate::theme::space(2.0))
                .py(crate::theme::space(1.0))
                .rounded(crate::theme::corner(6.0))
                .bg(theme::solid(theme::popup()))
                .border(crate::theme::line())
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
        .rounded(crate::theme::corner(6.0))
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
        .gap(crate::theme::space(1.0))
        .items_center()
        .rounded(crate::theme::corner(10.0))
        .bg(theme::solid(theme::toolbar()))
        .border(crate::theme::line())
        .border_color(theme::solid(theme::chrome_border()))
        .shadow_md()
        .text_color(theme::solid(theme::toolbar_text()))
}

/// The options of the tool in hand, beside its button: a column right of an
/// upright ribbon, a row under one floating at the top and over one at the
/// bottom.
fn options(
    model: &ControlsModel,
    place: ToolsPlace,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let panel = floats("tool-options").absolute();
    match place {
        ToolsPlace::Docked | ToolsPlace::FloatLeft => panel
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
    let upright = model.tools.upright();
    let groups = model.sections.iter().filter_map(|section| match section {
        ToolbarSection::Tools(tools) => Some(tools),
        ToolbarSection::Zoom(_) => None,
    });
    let mut out = Vec::new();
    for (index, group) in groups.enumerate() {
        if index > 0 {
            out.push(rule(!upright).into_any_element());
        }
        for button in group {
            let open = shown.filter(|_| button.active);
            // Upright, a button is named beside the ribbon, where its
            // options are when it has them open.
            let name = (upright && open.is_none())
                .then(|| beside(&button.id, hint(&button.label, button.chord)));
            out.push(
                div()
                    .group(SharedString::from(button.id.as_str().to_owned()))
                    .relative()
                    .child(tool(button, !upright))
                    .children(name)
                    .children(open.map(|shown| options(shown, model.tools, window, cx)))
                    .into_any_element(),
            );
        }
    }
    out
}

/// The tools docked: a strip filling the layout's `toolbar` block, which
/// the app counts as covered when it lies left of the canvas. The theme is
/// at its foot.
pub(super) fn docked(
    model: &ToolbarModel,
    shown: Option<&ControlsModel>,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    v_flex()
        .id("ribbon")
        .occlude()
        .size_full()
        .py(px(RIBBON_PAD))
        .gap(crate::theme::space(1.0))
        .items_center()
        .bg(super::material::panel_fill(theme::toolbar()))
        .border_r(crate::theme::line())
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

/// The tools floating, placed by [`super::mod_bar::floating`] over what
/// the canvas is seen through. It covers nothing the app counts.
pub(super) fn floating(
    model: &ToolbarModel,
    shown: Option<&ControlsModel>,
    edges: super::mod_bar::SlotEdges,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let upright = model.tools.upright();
    let ribbon = floats("ribbon")
        .when(upright, gpui_kit::Styled::flex_col)
        .children(tools(model, shown, window, cx))
        .child(rule(!upright))
        .child(theme_button(&model.theme, !upright));
    super::mod_bar::floating(ribbon.into_any_element(), model.tools, edges)
}

/// The zoom readout, for the end of the tab row.
pub(super) fn zoom(model: &ToolbarModel) -> Option<AnyElement> {
    model.sections.iter().find_map(|section| match section {
        ToolbarSection::Zoom(zoom) => Some(dropdown(zoom)),
        ToolbarSection::Tools(_) => None,
    })
}

/// The zoom readout, for a mod's `Zoom`: the narrow one where the ribbon
/// is docked.
pub(crate) fn zoom_readout(compact: bool) -> Option<AnyElement> {
    let (model, _) = crate::canvas::tool_models()?;
    model.sections.iter().find_map(|section| match section {
        ToolbarSection::Zoom(zoom) if compact => Some(dropdown_compact(zoom)),
        ToolbarSection::Zoom(zoom) => Some(dropdown(zoom)),
        ToolbarSection::Tools(_) => None,
    })
}

/// The options of the tool in hand, for a mod's `ToolOptions`, placed as
/// they are beside the app's own button.
pub(crate) fn tool_options(window: &mut Window, cx: &mut App) -> Option<AnyElement> {
    let (model, shown) = crate::canvas::tool_models()?;
    Some(options(&shown?, model.tools, window, cx).into_any_element())
}

/// Minimize and the lenses of the tab showing, for the end of the dock's bar.
pub(super) fn lens(model: &ToolbarModel, window: &mut Window, cx: &mut App) -> Vec<AnyElement> {
    (model.view.iter())
        .map(|model| control(model, window, cx))
        .collect()
}
