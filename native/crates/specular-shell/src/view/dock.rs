//! The dock: the third row of the chrome, with the controls of the tool in
//! hand or of the selection.

use gpui_kit::component::h_flex;
use gpui_kit::{App, IntoElement, ParentElement as _, Styled as _, Window, px};
use specular_interact::PopupModel;

use super::controls::control;
use crate::theme;

/// The dock's row, holding the controls of `model` from its left end. With
/// no model it is an empty bar, so the canvas under the chrome never moves.
pub(super) fn dock(
    model: Option<&PopupModel>,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let controls: Vec<_> = (model.iter())
        .flat_map(|model| &model.controls)
        .map(|model| control(model, window, cx))
        .collect();
    h_flex()
        .h(px(theme::DOCK_ROW))
        .flex_shrink_0()
        .px_3()
        .gap_1()
        .items_center()
        .children(controls)
}
