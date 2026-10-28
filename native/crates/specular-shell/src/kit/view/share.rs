//! The Share button at the right end of the tab row (ADR 0050): Share,
//! filled, while the canvas is not shared; Live, with the web viewer's
//! green dot, once it is, opening a row to copy each link and Stop sharing.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{Disableable as _, Sizable as _, h_flex};
use gpui_kit::{
    AnyElement, IntoElement as _, ParentElement as _, SharedString, Styled as _, div, px,
};
use specular_interact::{ShareButton, ShareState};

use super::controls::element_id;
use super::dropdown::opens;
use super::named::mark;
use super::run;
use crate::theme;

/// The live dot, `--live` in `web/style.css`.
const LIVE_DOT: u32 = 0x34c7_59ff;
const DOT: f32 = 7.0;
const RADIUS: f32 = 8.0;

pub(super) fn share(model: &ShareButton) -> AnyElement {
    let label = div()
        .text_xs()
        .child(SharedString::from(model.label.to_string()));
    if let Some(menu) = &model.menu {
        let dot = div()
            .size(px(DOT))
            .flex_shrink_0()
            .rounded_full()
            .bg(theme::solid(LIVE_DOT));
        let trigger = Button::new(element_id(&model.id.child("trigger")))
            .ghost()
            .small()
            .rounded(px(RADIUS))
            .child(h_flex().gap_1p5().items_center().child(dot).child(label));
        return opens(menu, trigger);
    }
    let button = Button::new(element_id(&model.id))
        .small()
        .rounded(px(RADIUS))
        .child(label)
        .child(crate::tip::over(model.id.as_str(), model.tip.to_string()))
        .child(mark(&model.id));
    let button = match model.state {
        ShareState::Off | ShareState::Failed => button.primary(),
        ShareState::Starting | ShareState::Live => button.ghost().disabled(true),
    };
    match model.action.clone() {
        Some(action) => button
            .on_click(move |_, window, cx| run(&action, window, cx))
            .into_any_element(),
        None => button.into_any_element(),
    }
}
