//! The panel's header and the list of threads: `PaneHeader`, `ThreadActions`
//! and `ThreadList` in `ChatPane.tsx`.
//!
//! With a thread open the title is the switcher: it opens a menu of the
//! canvas's threads. With none open the body is the list itself.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::TextareaState;
use gpui_kit::component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, Context, Entity, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, div, px,
};
use specular_interact::{Action, ChatModel, ThreadRow};

use super::date::short_date;
use super::run_keeping_field;
use crate::assets::ShellIcon;
use crate::theme;
use crate::view::{ShellView, run};

/// What the list says when the canvas has no thread: `ThreadList`'s own
/// line, the same words as an empty conversation's.
const NO_THREADS: &str = "Comment on the canvas to queue a draft, or type below and send.";

fn muted() -> gpui_kit::Hsla {
    theme::tinted(theme::TEXT_MUTED)
}

/// A 24 px square button of the header, with a 13 px glyph.
fn icon_button(id: &'static str, icon: impl Into<Icon>, tooltip: &'static str) -> Button {
    Button::new(id)
        .ghost()
        .xsmall()
        .size(px(24.0))
        .rounded(px(4.0))
        .tooltip(tooltip)
        .child(icon.into().size(px(13.0)))
}

/// One thread in the switcher's menu: its title, a tag while it is a draft,
/// and a check on the open one.
fn menu_item(row: &ThreadRow, input: &Entity<TextareaState>) -> PopupMenuItem {
    let (select, input) = (row.select.clone(), input.clone());
    let title = SharedString::from(row.title.clone());
    let item = if row.draft {
        PopupMenuItem::element(move |_, _| {
            h_flex()
                .gap_2()
                .items_center()
                .child(div().min_w_0().truncate().child(title.clone()))
                .child(Tag::secondary().xsmall().child("draft"))
        })
    } else {
        PopupMenuItem::new(title)
    };
    item.checked(row.active)
        .on_click(move |_, window, cx| run_keeping_field(&select, &input, window, cx))
}

impl ShellView {
    /// The title: plain over the list, the switcher over an open thread.
    fn chat_title(&self, model: &ChatModel) -> AnyElement {
        let title = SharedString::from(model.title.clone());
        if model.transcript.is_none() {
            return div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_weight(FontWeight::MEDIUM)
                .child(title)
                .into_any_element();
        }
        let (threads, input) = (model.threads.clone(), self.chat.input.clone());
        let switcher = Button::new("chat-threads")
            .ghost()
            .xsmall()
            .max_w_full()
            .tooltip("Switch thread")
            .dropdown_caret(true)
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .child(title),
            )
            .dropdown_menu(move |menu, _, _| {
                (threads.iter()).fold(menu.max_w(px(320.0)), |menu, row| {
                    menu.item(menu_item(row, &input))
                })
            });
        // The button's own inset would push the title off the header's
        // 12 px margin.
        h_flex()
            .flex_1()
            .min_w_0()
            .ml(px(-8.0))
            .child(switcher)
            .into_any_element()
    }

    /// The header strip: the title, then New, Close and Back.
    pub(super) fn chat_header(
        &self,
        model: &ChatModel,
        _cx: &mut Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let (new_thread, input) = (model.new_thread.clone(), self.chat.input.clone());
        let actions = h_flex()
            .flex_shrink_0()
            .gap(px(2.0))
            .items_center()
            .child(
                icon_button("chat-new", IconName::Plus, "New thread").on_click(
                    move |_, window, cx| run_keeping_field(&new_thread, &input, window, cx),
                ),
            )
            .when_some(model.close.clone(), |this, close| {
                this.child(
                    icon_button("chat-close", ShellIcon::Archive, "Close thread")
                        .on_click(move |_, window, cx| run(&close, window, cx)),
                )
            })
            .when_some(model.back.clone(), |this, back| {
                let input = self.chat.input.clone();
                this.child(
                    icon_button("chat-back", IconName::Close, "Back to threads").on_click(
                        move |_, window, cx| run_keeping_field(&back, &input, window, cx),
                    ),
                )
            });
        h_flex()
            .h(px(36.0))
            .flex_shrink_0()
            .px_3()
            .gap(px(6.0))
            .items_center()
            .border_b_1()
            .border_color(theme::solid(theme::PANEL_BORDER))
            .child(self.chat_title(model))
            .child(actions)
    }

    fn thread_row(&self, row: &ThreadRow) -> AnyElement {
        let (select, input) = (row.select.clone(), self.chat.input.clone());
        let close = row.close.clone();
        let id = SharedString::from(format!("chat-thread-{}", row.id.as_str()));
        let item = h_flex()
            .id(id.clone())
            .w_full()
            .px_3()
            .py(px(6.0))
            .gap_2()
            .items_center()
            .cursor_pointer()
            .hover(|this| this.bg(theme::solid(theme::ZINC_100)))
            // A right click is the menu's, not a choice of this thread.
            .on_click(move |event, window, cx| {
                if event.standard_click() {
                    run_keeping_field(&select, &input, window, cx);
                }
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(SharedString::from(row.title.clone())),
            )
            .when(row.draft, |this| {
                this.child(div().text_size(px(10.0)).text_color(muted()).child("DRAFT"))
            })
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(px(10.0))
                    .text_color(muted())
                    .child(SharedString::from(short_date(&row.updated_at))),
            );
        div()
            .id(SharedString::from(format!("{id}-menu")))
            .child(item)
            .context_menu(move |menu, _, _| {
                let close: Action = close.clone();
                menu.item(
                    PopupMenuItem::new("Close thread")
                        .on_click(move |_, window, cx| run(&close, window, cx)),
                )
            })
            .into_any_element()
    }

    /// The canvas's threads, newest first, when none is open.
    pub(super) fn thread_list(
        &self,
        model: &ChatModel,
        _cx: &mut Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        let rows: Vec<AnyElement> = (model.threads.iter())
            .map(|row| self.thread_row(row))
            .collect();
        v_flex()
            .id("chat-threads-list")
            .flex_1()
            .min_h_0()
            .py_1()
            .overflow_y_scroll()
            .when(rows.is_empty(), |this| {
                this.child(div().px_3().py_2().text_color(muted()).child(NO_THREADS))
            })
            .children(rows)
    }
}
