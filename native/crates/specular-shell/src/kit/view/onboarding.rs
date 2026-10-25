//! The first-run view (`src/renderer/onboarding`): shown in place of the
//! canvas while no space is open, from [`OnboardingModel`].
//!
//! The Prism turns at its head, the heading and its line are centred under
//! it, and the first two choices are buttons side by side. Any others are
//! quieter links below them.

mod prism;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::{
    Entity, FontWeight, IntoElement, ParentElement as _, SharedString, Styled as _, div, px,
};
use specular_interact::{OnboardingModel, SpaceChoice};

pub(crate) use self::prism::HeroPrism;
use super::run;
use crate::theme;

const WIDTH: f32 = 480.0;
const BUTTONS: usize = 2;

fn button(choice: &SpaceChoice) -> impl IntoElement + use<> {
    let action = choice.action.clone();
    let button = Button::new(choice.name)
        .large()
        // The Kit sizes a label by the button's size; this keeps the
        // large button's height with smaller text.
        .child(div().text_xs().child(choice.label))
        .tooltip(SharedString::from(choice.detail.clone()))
        .on_click(move |_, window, cx| run(&action, window, cx));
    if choice.primary {
        button.primary()
    } else {
        button.outline()
    }
}

fn link(choice: &SpaceChoice) -> impl IntoElement + use<> {
    let action = choice.action.clone();
    Button::new(choice.name)
        .ghost()
        .small()
        .child(choice.label)
        .tooltip(SharedString::from(choice.detail.clone()))
        .on_click(move |_, window, cx| run(&action, window, cx))
}

/// The view, over the whole window: nothing of an empty canvas shows
/// behind it.
pub(super) fn onboarding(model: &OnboardingModel, hero: Entity<HeroPrism>) -> impl IntoElement {
    let body = model.body.iter().map(|line| {
        div()
            .text_size(px(15.0))
            .line_height(px(22.0))
            .text_center()
            .text_color(theme::tinted(theme::text_muted()))
            .child(SharedString::from(line.clone()))
    });
    let (buttons, links) = model.choices.split_at(model.choices.len().min(BUTTONS));
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme::solid(theme::backdrop()))
        .child(
            v_flex()
                .w(px(WIDTH))
                .items_center()
                .child(hero)
                .child(
                    div()
                        .mt_6()
                        .text_size(px(28.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_center()
                        .child(model.title),
                )
                .child(v_flex().mt_2().gap_2().items_center().children(body))
                .child(
                    h_flex()
                        .mt_8()
                        .gap_3()
                        .justify_center()
                        .children(buttons.iter().map(button)),
                )
                .child(
                    h_flex()
                        .mt_3()
                        .gap_2()
                        .justify_center()
                        .children(links.iter().map(link)),
                )
                .children(model.note.clone().map(|note| {
                    div()
                        .mt_3()
                        .text_size(px(12.0))
                        .line_height(px(17.0))
                        .text_center()
                        .text_color(theme::tinted(theme::text_muted()))
                        .child(SharedString::from(note))
                })),
        )
}
