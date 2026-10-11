//! The cards under an agent's answer: what the turn made or edited. A card
//! whose item is on the canvas selects it and brings it into view.

use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, div, linear_color_stop, linear_gradient, px,
    rgba,
};
use specular_interact::ArtifactCard;

use crate::theme;
use crate::view::glyphs::{glyph, ink};
use crate::view::run;

/// A card's width: three fit across the panel at its starting width.
const WIDTH: f32 = 112.0;
/// The height of the picture at the top of a card.
const PICTURE: f32 = 64.0;
/// The bars of the sheet in the picture, as shares of its width.
const BARS: [f32; 3] = [1.0, 0.7, 0.85];

/// A small sheet with the kind's glyph and a few lines: a stand-in for the
/// thing, not a picture of its content.
fn picture(card: &ArtifactCard) -> impl IntoElement + use<> {
    let bar = theme::tinted(theme::text_muted()).opacity(0.35);
    let sheet = v_flex()
        .w(px(44.0))
        .h(px(48.0))
        .p(px(6.0))
        .gap(px(4.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(theme::solid(theme::input_border()))
        .bg(theme::solid(theme::input()))
        .child(glyph(card.glyph, ink(theme::text()), None, false, 12.0))
        .children(BARS.map(|share| {
            div()
                .h(px(3.0))
                .w(px(30.0 * share))
                .rounded(px(1.5))
                .bg(bar)
        }));
    h_flex()
        .h(px(PICTURE))
        .w_full()
        .items_center()
        .justify_center()
        .rounded(px(8.0))
        .bg(linear_gradient(
            180.0,
            linear_color_stop(rgba(theme::wash_from()), 0.0),
            linear_color_stop(rgba(theme::wash_to()), 1.0),
        ))
        .child(sheet)
}

fn card(id: SharedString, card: &ArtifactCard) -> AnyElement {
    let verb = if card.created { "New" } else { "Edited" };
    let body = v_flex()
        .id(id)
        .w(px(WIDTH))
        .flex_shrink_0()
        .p(px(4.0))
        .gap(px(4.0))
        .rounded(px(12.0))
        .border_1()
        .border_color(theme::solid(theme::input_border()))
        .child(picture(card))
        .child(
            v_flex()
                .px(px(4.0))
                .pb(px(2.0))
                .min_w_0()
                .child(
                    div()
                        .truncate()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme::solid(theme::text()))
                        .child(SharedString::from(card.label.clone())),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(11.0))
                        .text_color(theme::tinted(theme::text_muted()))
                        .child(SharedString::from(format!("{} · {verb}", card.detail))),
                ),
        );
    match card.focus.clone() {
        Some(focus) => body
            .cursor_pointer()
            .hover(|this| this.bg(theme::solid(theme::row_hover())))
            .on_click(move |_, window, cx| run(&focus, window, cx))
            .into_any_element(),
        // Not on the canvas: there is nowhere to go.
        None => body.opacity(0.7).into_any_element(),
    }
}

/// The cards of one answer, wrapping. `scope` keeps their ids apart from
/// another answer's.
pub(super) fn cards(scope: &str, cards: &[ArtifactCard]) -> impl IntoElement + use<> {
    h_flex()
        .flex_wrap()
        .gap(px(8.0))
        .when(cards.is_empty(), gpui_kit::Styled::hidden)
        .children((cards.iter().enumerate()).map(|(index, it)| {
            card(
                SharedString::from(format!("chat-artifact-{scope}-{index}")),
                it,
            )
        }))
}
