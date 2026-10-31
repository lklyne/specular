//! The Share button at the right end of the tab row (ADR 0050), and its
//! popover: who the link lets in, and Copy link. The popover's content is
//! looked up in the models each frame, as an open dropdown's is.
//!
//! The access choices open in place, under their row, rather than in a
//! popover of their own: a second popover's press would land outside the
//! first and close it.

use std::cell::Cell;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::{Disableable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, IntoElement as _, ParentElement as _, SharedString, Styled as _, div, px,
};
use specular_interact::{DropdownSection, Icon, ShareButton};

use super::controls::element_id;
use super::glyphs::{glyph, ink};
use super::named::mark;
use super::run;
use crate::{canvas, theme};

const RADIUS: f32 = 8.0;
/// The popover's width, Paper's.
const WIDTH: f32 = 300.0;
const ROW: f32 = 34.0;

thread_local! {
    /// Whether the access choices are open under their row.
    static CHOOSING: Cell<bool> = const { Cell::new(false) };
}

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
                    (access.summary.text.as_ref())
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
        )
        .child(mark(&access.id))
        .on_click(|_, window, _| {
            CHOOSING.set(!CHOOSING.get());
            window.refresh();
        });
    let choices = CHOOSING.get().then(|| {
        let options = access.content.iter().flat_map(|section| match section {
            DropdownSection::Options { options, .. } => options.as_slice(),
            DropdownSection::Controls(_) => &[],
        });
        v_flex().children(options.map(|option| {
            let action = option.action.clone();
            Button::new(element_id(&option.id))
                .ghost()
                .w_full()
                .h(px(ROW - 4.0))
                .rounded(px(RADIUS - 2.0))
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .child(text(option.label.to_string()))
                        .children(option.selected.then(|| {
                            glyph(Icon::Check, ink(theme::text_muted()), None, false, 12.0)
                        })),
                )
                .child(mark(&option.id))
                .on_click(move |_, window, cx| {
                    CHOOSING.set(false);
                    run(&action, window, cx);
                })
        }))
    });
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
        .child(trigger)
        .children(choices)
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
        .on_open_change(|_, _, _| CHOOSING.set(false))
        .content(move |_, _, _| {
            canvas::models().map_or_else(
                || div().into_any_element(),
                |models| content(&models.toolbar.share),
            )
        })
        .into_any_element()
}
