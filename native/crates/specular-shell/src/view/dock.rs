//! The dock: a bar under the tab row, there while it has something to hold.
//! It has the controls of the selection, and in a tab that shows its item
//! the lens at its far end.

use gpui_kit::component::h_flex;
use gpui_kit::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, Window, px,
};
use specular_interact::{ControlsModel, ToolbarModel};

use super::controls::control;
use super::toolbar;
use crate::theme;

/// The dock's bar across the top of what the canvas is seen through, which
/// starts `left` from the slot's edge: the controls of `model` from its
/// left end, taking the room the lens leaves. `None` when neither has
/// anything to show.
pub(super) fn dock(
    model: Option<&ControlsModel>,
    toolbar: &ToolbarModel,
    left: f32,
    window: &mut Window,
    cx: &mut App,
) -> Option<impl IntoElement + use<>> {
    if model.is_none() && toolbar.view.is_empty() {
        return None;
    }
    let controls: Vec<_> = (model.iter())
        .flat_map(|model| &model.controls)
        .map(|model| control(model, window, cx))
        .collect();
    Some(
        h_flex()
            .id("dock")
            .occlude()
            .absolute()
            .top(px(theme::CHROME_HEIGHT))
            .left(px(left))
            .right_0()
            .h(px(theme::DOCK_ROW))
            .pl_2()
            .pr_3()
            .gap_1()
            .items_center()
            .bg(theme::solid(theme::toolbar()))
            .border_b_1()
            .border_color(theme::solid(theme::toolbar_border()))
            .child(
                // A focused field's ring is drawn outside it, so the row
                // that clips its controls is the bar's height and padded.
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .px_1()
                    .gap_1()
                    .items_center()
                    .overflow_hidden()
                    .children(controls),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .items_center()
                    .children(toolbar::lens(toolbar, window, cx)),
            ),
    )
}
