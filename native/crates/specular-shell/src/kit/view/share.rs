//! The Share button at the right end of the tab row (ADR 0050), and its
//! popover: who the link lets in, and Copy link. The popover's content is
//! looked up in the models each frame, as an open dropdown's is.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::{Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, IntoElement as _, ParentElement as _, SharedString, Styled as _, div, px,
};
use specular_interact::{Icon, ShareButton};

use super::controls::element_id;
use super::dropdown::opens;
use super::glyphs::{glyph, ink};
use super::named::mark;
use super::run;
use crate::{canvas, theme};

const RADIUS: f32 = 8.0;
/// The popover's width, Paper's.
const WIDTH: f32 = 300.0;
const ROW: f32 = 34.0;

fn text(label: String) -> impl gpui_kit::IntoElement {
    div().text_sm().child(SharedString::from(label))
}

/// The popover's rows for `model`.
fn content(model: &ShareButton) -> AnyElement {
    let access = &model.access;
    let trigger = Button::new(element_id(&access.id.child("trigger")))
        .outline()
        .w_full()
        .h(px(ROW))
        .rounded(px(RADIUS))
        .child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(text(
                    access
                        .summary
                        .text
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                ))
                .child(glyph(
                    Icon::ChevronDown,
                    ink(theme::text_muted()),
                    None,
                    false,
                    12.0,
                )),
        );
    let action = model.copy.action.clone();
    let copy = Button::new(element_id(&model.copy.id))
        .primary()
        .w_full()
        .h(px(ROW))
        .rounded(px(RADIUS))
        .disabled(!model.copy.enabled)
        .child(text(model.copy.label.to_string()))
        .child(mark(&model.copy.id))
        .on_click(move |_, window, cx| run(&action, window, cx));
    v_flex()
        .w(px(WIDTH))
        .p_1()
        .gap_2()
        .child(opens(access, trigger))
        .child(copy)
        .children(model.note.as_ref().map(|note| {
            div()
                .text_xs()
                .text_color(theme::tinted(theme::text_muted()))
                .child(SharedString::from(note.to_string()))
        }))
        .into_any_element()
}

pub(super) fn share(model: &ShareButton) -> AnyElement {
    let trigger = Button::new(element_id(&model.id.child("trigger")))
        .primary()
        .small()
        .rounded(px(RADIUS))
        .child(div().text_xs().child("Share"))
        .child(mark(&model.id));
    Popover::new(element_id(&model.id))
        .p_2()
        .trigger(trigger)
        .content(move |_, _, _| {
            canvas::models().map_or_else(
                || div().into_any_element(),
                |models| content(&models.toolbar.share),
            )
        })
        .into_any_element()
}
