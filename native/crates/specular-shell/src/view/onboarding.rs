//! The first-run view (`src/renderer/onboarding`): shown in place of the
//! canvas while no space is open, from [`OnboardingModel`].

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::v_flex;
use gpui_kit::{FontWeight, IntoElement, ParentElement as _, SharedString, Styled as _, div, px};
use specular_interact::{OnboardingModel, SpaceChoice};

use super::run;
use crate::theme;

const WIDTH: f32 = 440.0;

fn choice(choice: &SpaceChoice) -> impl IntoElement + use<> {
    let action = choice.action.clone();
    let button = Button::new(choice.name).w_full().child(choice.label);
    let button = if choice.primary {
        button.primary()
    } else {
        button.outline()
    };
    v_flex()
        .gap_1()
        .child(button.on_click(move |_, window, cx| run(&action, window, cx)))
        .child(
            div()
                .px(px(2.0))
                .text_size(px(11.0))
                .text_color(theme::tinted(theme::TEXT_MUTED))
                .child(SharedString::from(choice.detail.clone())),
        )
}

/// The view, over the whole window: nothing of an empty canvas shows
/// behind it.
pub(super) fn onboarding(model: &OnboardingModel) -> impl IntoElement + use<> {
    let body = model.body.iter().map(|paragraph| {
        div()
            .text_size(px(13.0))
            .line_height(px(19.0))
            .text_color(theme::tinted(theme::TEXT_MUTED))
            .child(SharedString::from(paragraph.clone()))
    });
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme::solid(theme::PANEL))
        .child(
            v_flex()
                .w(px(WIDTH))
                .gap_3()
                .child(
                    div()
                        .text_size(px(24.0))
                        .font_weight(FontWeight::BOLD)
                        .child(model.title),
                )
                .children(body)
                .child(
                    v_flex()
                        .mt_3()
                        .gap_4()
                        .children(model.choices.iter().map(choice)),
                ),
        )
}
