//! What hangs above and below the composer's field: the comment draft's
//! chip (`CommentDraftChip`), the open-comments row (`OpenComments`), the
//! queued messages (`QueuedComments`) and the quiet labels of the bottom
//! row (`composerChipClass`).

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled, div, px, rgba,
};
use specular_interact::{
    AutoChip, DraftChip, DraftKind, Event, OpenComments, PillKind, QueuedChip,
};

use crate::assets::ShellIcon;
use crate::canvas;
use crate::theme;
use crate::view::run;
pub(super) fn muted() -> gpui_kit::Hsla {
    theme::tinted(theme::text_muted())
}

/// A 16 px round remove button that shows while `group` is hovered.
pub(super) fn remove_button(
    id: SharedString,
    group: SharedString,
    fill: u32,
    ink: u32,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .absolute()
        .size(px(16.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(rgba(fill))
        .text_color(rgba(ink))
        .cursor_pointer()
        .invisible()
        .group_hover(group, Styled::visible)
        .child(Icon::new(IconName::Close).size(px(10.0)))
}

/// The open comment draft as a removable pill (`CommentDraftChip`).
pub(super) fn draft_chip(draft: &DraftChip) -> impl IntoElement + use<> {
    let remove = draft.remove.clone();
    let icon = match draft.kind {
        DraftKind::Region | DraftKind::Selection => ShellIcon::SquareDashed,
        DraftKind::Element | DraftKind::Point => ShellIcon::MessageSquare,
    };
    h_flex().mb(px(6.0)).child(
        h_flex()
            .group("chat-draft")
            .relative()
            .max_w_full()
            .min_w_0()
            .gap(px(6.0))
            .items_center()
            .rounded_full()
            .border_1()
            .border_color(theme::solid(theme::zinc_300()))
            .bg(theme::solid(theme::zinc_100()))
            .py_1()
            .pl_2()
            .pr_6()
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .child(Icon::new(icon).size(px(12.0)).text_color(muted()))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .child(SharedString::from(draft.label.clone())),
            )
            .child(
                remove_button(
                    "chat-draft-remove".into(),
                    "chat-draft".into(),
                    0x0000_001a,
                    0x3030_3094,
                )
                .right_1()
                .top(px(4.0))
                .tooltip(|window, cx| Tooltip::new("Remove comment draft").build(window, cx))
                .on_click(move |_, _, _| canvas::dispatch(Event::Action(remove.clone()))),
            ),
    )
}

/// How many of the thread's sent comments are still open, and the one
/// action that resolves them (`OpenComments`).
pub(super) fn open_comments(open: &OpenComments) -> impl IntoElement + use<> {
    let resolve = open.resolve.clone();
    h_flex()
        .px(px(2.0))
        .pb_1()
        .gap_2()
        .items_center()
        .justify_between()
        .text_size(px(11.0))
        .font_weight(FontWeight::MEDIUM)
        .child(div().text_color(muted()).child(SharedString::from(format!(
            "Thread comments {}",
            open.count
        ))))
        .child(
            Button::new("chat-resolve")
                .ghost()
                .xsmall()
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .child("Resolve comments"),
                )
                .on_click(move |_, window, cx| run(&resolve, window, cx)),
        )
}

/// What waits to be sent, stacked above the field (`QueuedComments`).
pub(super) fn queued(chips: &[QueuedChip]) -> impl IntoElement + use<> {
    v_flex().gap_1().pb_1().children(chips.iter().map(|chip| {
        let icon = if chip.annotation.is_some() || !is_image_label(&chip.text) {
            ShellIcon::MessageSquare
        } else {
            ShellIcon::Image
        };
        h_flex()
            .items_start()
            .gap(px(6.0))
            .rounded(px(8.0))
            .px(px(6.0))
            .py_1()
            .text_size(px(12.0))
            .line_height(px(20.0))
            .bg(theme::tinted(theme::queued()))
            .child(
                Icon::new(icon)
                    .size(px(11.0))
                    .mt(px(5.0))
                    .flex_shrink_0()
                    .text_color(muted()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .line_clamp(3)
                    .child(SharedString::from(chip.text.clone())),
            )
    }))
}

/// Whether a queued chip's words are the stand-in for a message that is
/// only images: `Image` or `N images`.
fn is_image_label(text: &str) -> bool {
    text == "Image"
        || text
            .strip_suffix(" images")
            .is_some_and(|count| count.parse::<usize>().is_ok())
}

/// A quiet label of the bottom row that takes on a pill when hovered
/// (`composerChipClass`).
pub(super) fn chip(id: &'static str, icon: Icon, label: &str) -> gpui_kit::Stateful<gpui_kit::Div> {
    h_flex()
        .id(id)
        .min_w_0()
        .gap_1()
        .items_center()
        .rounded_full()
        .px(px(6.0))
        .py(px(2.0))
        .text_size(px(11.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(muted())
        .hover(|this| this.bg(theme::tinted(theme::chip_hover())))
        .child(icon.size(px(11.0)).flex_shrink_0())
        .child(
            div()
                .min_w_0()
                .truncate()
                .child(SharedString::from(label.to_owned())),
        )
}

pub(super) fn pill_icon(kind: PillKind) -> Icon {
    match kind {
        PillKind::Dom => Icon::new(ShellIcon::Code),
        PillKind::Selection => Icon::new(ShellIcon::SquareDashedMousePointer),
        PillKind::Comment | PillKind::Canvas => Icon::new(IconName::File),
    }
}

/// The send mode of the origin this turn writes for: `Auto` sends each
/// comment as it is placed, `Queue` holds them until the person sends.
pub(super) fn auto_chip(auto: &AutoChip) -> impl IntoElement + use<> {
    let toggle = auto.toggle.clone();
    let (icon, label, tip) = if auto.on {
        (
            ShellIcon::Zap,
            "Auto",
            format!(
                "Auto for {}: each comment is sent as soon as it is placed. Click to queue instead.",
                auto.origin
            ),
        )
    } else {
        (
            ShellIcon::ListEnd,
            "Queue",
            format!(
                "Queue for {}: comments wait here until you send. Click to send automatically.",
                auto.origin
            ),
        )
    };
    let tip = SharedString::from(tip);
    chip("chat-auto", Icon::new(icon), label)
        .cursor_pointer()
        .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
        .on_click(move |_, _, _| canvas::dispatch(Event::Action(toggle.clone())))
}
