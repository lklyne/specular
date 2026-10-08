//! The composer: `Composer` in `ChatPane.tsx` and its chips.
//!
//! One rounded box holding, top to bottom: the comment draft's chip, the
//! open-comments row, the queued messages, the pasted images, the field,
//! and a row with the context pill, the folder and Send.
//!
//! Enter sends and Shift+Enter opens a line. Escape drops an open comment
//! draft and leaves what was typed; with no draft it hands the keys back
//! to the canvas.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Escape, Textarea};
use gpui_kit::component::message_scroller::MessageScrollerState;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Disableable as _, Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ObjectFit, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, StyledImage as _, Window, div, img,
    px,
};
use specular_interact::{Action, ChatAction, Composer, Event};

use super::chips::{
    auto_chip, chip, draft_chip, muted, open_comments, pill_icon, queued, remove_button,
};
use crate::canvas;
use crate::theme;
use crate::view::{ShellView, focus_canvas};

/// The field is two lines tall when empty and grows to six: `min-h-[48px]`
/// and `max-h-[160px]` at a 24 px line.
pub(super) const MIN_ROWS: usize = 2;
pub(super) const MAX_ROWS: usize = 6;
const LINE: f32 = 24.0;

impl ShellView {
    /// Sends what the composer holds: the field's text and the pasted
    /// images. The field is emptied and keeps the keys.
    pub(super) fn send_chat(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let text = self.chat.input.read(cx).value().trim().to_owned();
        let can_send_empty =
            canvas::models().is_some_and(|models| models.chat.composer.can_send_empty);
        if text.is_empty() && self.chat.images.is_empty() && !can_send_empty {
            return;
        }
        let images = (self.chat.images.drain(..))
            .map(|pasted| pasted.upload)
            .collect();
        canvas::dispatch(Event::Action(Action::Chat(ChatAction::Send {
            text,
            images,
        })));
        self.chat.input.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        // What was just sent is at the end.
        self.chat
            .scroller
            .update(cx, MessageScrollerState::scroll_to_end);
        cx.notify();
    }

    /// Escape in the field: drops the comment draft, or with none gives the
    /// keys back to the canvas. What was typed stays.
    fn leave_composer(window: &mut Window, cx: &mut Context<'_, Self>) {
        let drafting = canvas::models().is_some_and(|models| models.chat.composer.draft.is_some());
        if drafting {
            canvas::dispatch(Event::Action(Action::Chat(ChatAction::DropDraft)));
        } else {
            focus_canvas(window, cx);
        }
    }

    /// Thumbnails of the pasted images, each removable (`PastedImages`).
    fn pasted_images(&self, cx: &mut Context<'_, Self>) -> Option<impl IntoElement + use<>> {
        if self.chat.images.is_empty() {
            return None;
        }
        let thumbnails = (self.chat.images.iter().enumerate()).map(|(index, pasted)| {
            let group = SharedString::from(format!("chat-pasted-{index}"));
            div()
                .group(group.clone())
                .relative()
                .size(px(48.0))
                .flex_shrink_0()
                .overflow_hidden()
                .rounded(px(6.0))
                .border_1()
                .border_color(theme::solid(theme::zinc_300()))
                .child(
                    img(pasted.thumbnail.clone())
                        .size_full()
                        .object_fit(ObjectFit::Cover),
                )
                .child(
                    remove_button(
                        SharedString::from(format!("chat-pasted-remove-{index}")),
                        group,
                        0x0000_00b3,
                        0xffff_ffff,
                    )
                    .right(px(2.0))
                    .top(px(2.0))
                    .tooltip(|window, cx| Tooltip::new("Remove image").build(window, cx))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if index < this.chat.images.len() {
                            this.chat.images.remove(index);
                            cx.notify();
                        }
                    })),
                )
        });
        Some(
            h_flex()
                .flex_wrap()
                .gap(px(6.0))
                .pb(px(6.0))
                .children(thumbnails),
        )
    }

    /// The round Send button: filled once there is something to send.
    fn send_button(ready: bool, cx: &mut Context<'_, Self>) -> AnyElement {
        let arrow = div().text_size(px(12.0)).child("↑");
        let button = Button::new("chat-send")
            .size(px(28.0))
            .rounded(px(14.0))
            .flex_shrink_0()
            .tooltip("Send")
            .child(arrow)
            .on_click(cx.listener(|this, _, window, cx| this.send_chat(window, cx)));
        if ready {
            button.primary().into_any_element()
        } else {
            button
                .ghost()
                .disabled(true)
                .bg(theme::solid(theme::zinc_100()))
                .text_color(muted())
                .into_any_element()
        }
    }

    /// The composer for `model`.
    pub(super) fn composer(
        &mut self,
        model: &Composer,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement + use<> {
        if self.chat.placeholder != model.placeholder {
            self.chat.placeholder.clone_from(&model.placeholder);
            let placeholder = SharedString::from(model.placeholder.clone());
            self.chat.input.update(cx, |state, cx| {
                state.set_placeholder(placeholder, window, cx);
            });
        }
        let typed = !self.chat.input.read(cx).value().trim().is_empty();
        let ready = typed || !self.chat.images.is_empty() || model.can_send_empty;

        let view = cx.entity().downgrade();
        // The smallest size has the smallest inset around the text; the type
        // is the composer's own.
        let field = Textarea::new(&self.chat.input)
            .xsmall()
            .appearance(false)
            .bordered(false)
            .text_size(px(14.0))
            .line_height(px(LINE))
            .on_paste(move |item, _, cx| {
                view.update(cx, |this, cx| this.take_pasted(item, cx))
                    .unwrap_or(false)
            });
        let folder = model.folder.as_ref().map(|folder| {
            let target = model.folder_path.as_ref().unwrap_or(folder);
            let written = SharedString::from(format!("Changes are written to {target}"));
            chip("chat-folder", Icon::new(IconName::FolderOpen), folder)
                .tooltip(move |window, cx| Tooltip::new(written.clone()).build(window, cx))
        });
        let bottom = h_flex()
            .min_w_0()
            .gap_1()
            .pt(px(2.0))
            .items_center()
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .items_center()
                    .child(chip(
                        "chat-pill",
                        pill_icon(model.pill.kind),
                        &model.pill.label,
                    ))
                    .children(folder)
                    .children(model.auto.as_ref().map(auto_chip)),
            )
            .child(Self::send_button(ready, cx));

        let focus_field = cx.listener(|this, _: &gpui_kit::ClickEvent, window, cx| {
            this.focus_composer(window, cx);
        });
        let box_ = v_flex()
            .id("chat-composer")
            .rounded(px(16.0))
            .border_1()
            .border_color(theme::solid(theme::zinc_300()))
            .bg(theme::solid(theme::zinc_50()))
            .px_2()
            .py(px(6.0))
            // The field has had its turn by the time Escape reaches here: an
            // input method's composition is closed first.
            .on_action(cx.listener(|_, _: &Escape, window, cx| {
                Self::leave_composer(window, cx);
            }))
            .children(model.draft.as_ref().map(draft_chip))
            .children(model.open_comments.as_ref().map(open_comments))
            .when(!model.queued.is_empty(), |this| {
                this.child(queued(&model.queued))
            })
            .children(self.pasted_images(cx))
            .child(
                div()
                    .id("chat-field")
                    .min_h(px(LINE * MIN_ROWS as f32))
                    .p(px(2.0))
                    .cursor_text()
                    .on_click(focus_field)
                    .child(field),
            )
            .child(bottom);
        div()
            .flex_shrink_0()
            .p_2()
            .border_t_1()
            .border_color(theme::solid(theme::zinc_200()))
            .child(box_)
    }
}
