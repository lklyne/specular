//! Tooltips at the size of panel text. The Kit's own are a step larger and
//! a Kit button builds its tooltip itself, so a button carries [`over`] as a
//! child and leaves the Kit's off.

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{
    AnyView, App, Div, InteractiveElement as _, SharedString, Stateful,
    StatefulInteractiveElement as _, Styled as _, Window, div,
};

/// Builds the tooltip reading `text`, for an element's `.tooltip(..)`.
pub(crate) fn view(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView {
    let text: SharedString = text.into();
    move |window, cx| Tooltip::new(text.clone()).text_xs().build(window, cx)
}

/// A child that covers its parent and shows `text` on hover. `id` is the
/// parent's own, so two tooltips never share one.
pub(crate) fn over(id: &str, text: impl Into<SharedString>) -> Stateful<Div> {
    let text: SharedString = text.into();
    div()
        .id(SharedString::from(format!("tip-{id}")))
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .tooltip(view(text))
}
