//! The popup hung from the toolbar: the controls of the tool in hand. A
//! popup beside a canvas item is not here. It is drawn in the canvas's own
//! pass, where it moves in step with the item.

use std::rc::Rc;

use gpui_kit::component::h_flex;
use gpui_kit::{
    App, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render, Styled as _,
    Window, div, px,
};
use specular_interact::{PopupAnchor, PopupModel};

use super::controls::control;
use super::named;
use crate::models::same;
use crate::theme;
use crate::ui_stats::{self, Part};

/// The popup hung from the toolbar, a view of its own: it is built again
/// when its model changed.
pub(super) struct PopupView {
    model: Option<Rc<PopupModel>>,
}

impl PopupView {
    pub(super) fn new() -> Self {
        Self { model: None }
    }

    /// Shows `model`, and draws again if it is not the one shown.
    pub(super) fn set(&mut self, model: Option<&Rc<PopupModel>>, cx: &mut Context<'_, Self>) {
        if !same(self.model.as_ref(), model) {
            self.model = model.cloned();
            cx.notify();
        }
    }
}

impl Render for PopupView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        ui_stats::built(Part::Popup);
        named::begin(Part::Popup);
        match self.model.clone() {
            Some(model) => tool_popup(&model, window, cx).into_any_element(),
            None => div().into_any_element(),
        }
    }
}

/// The popup of the tool in hand, centred under the toolbar. The root view
/// gives it everything under the toolbar to be laid out in.
fn tool_popup(model: &PopupModel, window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let gap = match model.anchor {
        PopupAnchor::Toolbar { gap } | PopupAnchor::Canvas { gap, .. } => gap,
        PopupAnchor::Point(_) => 0.0,
    };
    let controls: Vec<_> = (model.controls.iter())
        .map(|model| control(model, window, cx))
        .collect();
    // The row spans the window only to centre the popup. It takes no
    // pointer events, so the canvas under its empty ends still does.
    h_flex()
        .size_full()
        .pt(px(gap))
        .items_start()
        .justify_center()
        .child(
            h_flex()
                .id("tool-popup")
                .occlude()
                .gap_1()
                .items_center()
                .p_1()
                .rounded(px(10.0))
                .border_1()
                .border_color(theme::solid(theme::CHROME_BORDER))
                .bg(theme::solid(theme::POPUP))
                .shadow_md()
                .children(controls),
        )
}
