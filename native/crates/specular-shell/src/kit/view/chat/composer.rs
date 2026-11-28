//! The composer: `Composer` in `ChatPane.tsx` and its chips.
//!
//! One rounded box holding, top to bottom: the comment draft's chip, the
//! open-comments row, the queued messages, the pasted images, the field,
//! and a row with the context pill, the folder and Send.
//!
//! Enter sends and Shift+Enter opens a line. Escape drops an open comment
//! draft and leaves what was typed; with no draft it hands the keys back
//! to the canvas.

use std::rc::Rc;

use gpui_kit::component::input::Escape;
use gpui_kit::component::message_scroller::MessageScrollerState;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _, SharedString, Styled as _,
    Window, div,
};
use specular_interact::{Action, ChatAction, Composer, Event};

use super::chips::{draft_chip, open_comments, queued};
use super::composer_box::{self, ComposerBox, OnPaste, OnRemove, OnSend};
use crate::canvas;
use crate::theme;
use crate::view::{ShellView, focus_canvas};

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
            thread: None,
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
        let on_paste: OnPaste = {
            let view = view.clone();
            Rc::new(move |item, _, cx| {
                view.update(cx, |this, cx| this.take_pasted(item, cx))
                    .unwrap_or(false)
            })
        };
        let on_send: OnSend = {
            let view = view.clone();
            Rc::new(move |window, cx| {
                let _ = view.update(cx, |this, cx| this.send_chat(window, cx));
            })
        };
        let on_remove: OnRemove = Rc::new(move |index, _, cx| {
            let _ = view.update(cx, |this, cx| {
                if index < this.chat.images.len() {
                    this.chat.images.remove(index);
                    cx.notify();
                }
            });
        });
        let parts = ComposerBox {
            model,
            input: &self.chat.input,
            images: &self.chat.images,
            ready,
        };
        let box_ = composer_box::frame("chat-composer")
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
            .children(parts.thumbnails(&on_remove))
            .child(parts.field(on_paste))
            .child(parts.chips(on_send));
        div()
            .flex_shrink_0()
            .p(crate::theme::space(2.0))
            .border_t(crate::theme::line())
            .border_color(theme::solid(theme::panel_border()))
            .child(box_)
    }
}
