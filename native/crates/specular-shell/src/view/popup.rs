//! The popup hung from the toolbar: the controls of the tool in hand. A
//! popup beside a canvas item is not here. It is drawn in the canvas's own
//! pass, where it moves in step with the item.

use gpui_kit::component::h_flex;
use gpui_kit::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, div, px,
};
use specular_interact::{PopupAnchor, PopupModel};

use super::controls::control;
use crate::theme;

/// The popup of the tool in hand, centred under the toolbar.
pub(super) fn tool_popup(model: &PopupModel, _cx: &App) -> impl IntoElement {
    let gap = match model.anchor {
        PopupAnchor::Toolbar { gap } | PopupAnchor::Canvas { gap, .. } => gap,
        PopupAnchor::Point(_) => 0.0,
    };
    // The row spans the window only to centre the popup. It takes no
    // pointer events, so the canvas under its empty ends still does.
    h_flex()
        .absolute()
        .top(px(theme::TOOLBAR_HEIGHT + gap))
        .left_0()
        .right_0()
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
                .children(model.controls.iter().map(control)),
        )
        .child(div())
}
