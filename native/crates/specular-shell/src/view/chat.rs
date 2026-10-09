//! The right panel, from [`ChatModel`]: the canvas's agent threads, the
//! open thread's conversation, and the composer.
//!
//! ```text
//! ┌ header: title (the thread switcher), new, close, back ┐
//! ├ the thread list, or the open thread's transcript      ┤
//! ├ composer: chips, pasted images, the field, send       ┘
//! ```
//!
//! Every control sends the model's own `Action`. The view keeps only what a
//! view must: the field's text, the images pasted into it, where the
//! transcript is scrolled, whether the run log is open, and a drag of the
//! panel's edge.

mod chips;
mod composer;
mod date;
mod header;
mod paste;
mod run_bar;
mod transcript;

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::component::input::{InputEvent, TextareaState};
use gpui_kit::component::message_scroller::MessageScrollerState;
use gpui_kit::component::v_flex;
use gpui_kit::{
    App, AppContext as _, Context, CursorStyle, DispatchPhase, Entity, InteractiveElement as _,
    IntoElement, MouseButton, MouseMoveEvent, MouseUpEvent, ParentElement as _, ScrollHandle,
    Styled as _, Subscription, Window, canvas, div, px,
};
use specular_interact::{Action, ChatAction, ChatModel, Event};

use self::paste::Pasted;
use self::transcript::Shown;
use super::{ShellView, focus_canvas};
use crate::canvas as app_canvas;
use crate::theme;

/// The strip along the panel's left edge that resizes it,
/// `DEVTOOLS_RESIZE_HANDLE_WIDTH`.
const RESIZE_HANDLE: f32 = 12.0;

/// What the right panel keeps between frames.
pub(super) struct ChatUi {
    /// The composer's field.
    input: Entity<TextareaState>,
    _events: Subscription,
    /// Images pasted into the field, sent with the next message.
    images: Vec<Pasted>,
    /// Where the transcript is scrolled, and whether it follows its end.
    scroller: Entity<MessageScrollerState>,
    /// What the scroller was last told it shows.
    shown: Shown,
    /// Whether the run bar's log is open.
    log_open: bool,
    /// The run log's own scroll.
    log_scroll: ScrollHandle,
    /// Whether the panel's edge is being dragged.
    resizing: Rc<Cell<bool>>,
    /// The comment draft the field was last focused for.
    draft: Option<String>,
    /// The hint the field was last given.
    placeholder: String,
}

impl ChatUi {
    pub(super) fn new(window: &mut Window, cx: &mut Context<'_, ShellView>) -> Self {
        // Enter sends and Shift+Enter opens a line: the field reports a
        // plain Enter and inserts nothing for it.
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(composer::MIN_ROWS, composer::MAX_ROWS)
                .submit_on_enter(true)
        });
        let events =
            cx.subscribe_in(
                &input,
                window,
                |this, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { shift: false, .. } => this.send_chat(window, cx),
                    // Send is live only while there is something to send.
                    InputEvent::Change => cx.notify(),
                    InputEvent::PressEnter { .. } | InputEvent::Focus | InputEvent::Blur => {}
                },
            );
        Self {
            input,
            _events: events,
            images: Vec::new(),
            scroller: cx.new(|cx| MessageScrollerState::new(0, cx)),
            shown: Shown::default(),
            log_open: false,
            log_scroll: ScrollHandle::new(),
            resizing: Rc::new(Cell::new(false)),
            draft: None,
            placeholder: String::new(),
        }
    }
}

/// Runs a panel control's `action` and leaves the keys in the composer's
/// field: the panel has one field, and what is clicked beside it (a thread,
/// New) is followed by typing.
fn run_keeping_field(
    action: &Action,
    input: &Entity<TextareaState>,
    window: &mut Window,
    cx: &mut App,
) {
    app_canvas::dispatch(Event::Action(action.clone()));
    input.update(cx, |state, cx| state.focus(window, cx));
}

impl ShellView {
    /// Focuses the composer's field.
    fn focus_composer(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.chat
            .input
            .update(cx, |state, cx| state.focus(window, cx));
    }

    /// Whether the keys are in the composer's field.
    pub(super) fn composer_focused(&self, window: &Window, cx: &App) -> bool {
        use gpui_kit::Focusable as _;
        self.chat.input.focus_handle(cx).is_focused(window)
    }

    /// The right panel for `model`, when it is open.
    pub(super) fn chat_panel(
        &mut self,
        model: &ChatModel,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Option<impl IntoElement + use<>> {
        self.follow_draft(model, window, cx);
        if !model.visible {
            // A hidden field must not keep the keys.
            if self.composer_focused(window, cx) {
                focus_canvas(window, cx);
            }
            return None;
        }
        let body = match &model.transcript {
            Some(transcript) => self.transcript(model, transcript, cx).into_any_element(),
            None => self.thread_list(model, cx).into_any_element(),
        };
        Some(
            v_flex()
                .id("chat-panel")
                .occlude()
                .w(px(model.width))
                .h_full()
                .flex_shrink_0()
                .pt(px(theme::CHROME_HEIGHT))
                .bg(theme::solid(theme::panel()))
                .border_l_1()
                .border_color(theme::solid(theme::chrome_border()))
                .child(self.chat_header(model, cx))
                .child(body)
                .child(self.composer(&model.composer, window, cx)),
        )
    }

    /// Gives the field the keys when a comment draft appears, so it can be
    /// typed at once.
    fn follow_draft(&mut self, model: &ChatModel, window: &mut Window, cx: &mut Context<'_, Self>) {
        let draft = (model.composer.draft.as_ref()).map(|draft| draft.label.clone());
        if draft == self.chat.draft {
            return;
        }
        self.chat.draft = draft;
        if self.chat.draft.is_some() && model.visible {
            self.focus_composer(window, cx);
        }
    }

    /// The strip over the panel's left edge. Dragging it asks for the width
    /// that puts the edge under the pointer, and the model keeps that within
    /// its limits.
    pub(super) fn chat_resize_handle(&self, model: &ChatModel) -> Option<impl IntoElement + use<>> {
        if !model.visible {
            return None;
        }
        let (press, drag, lift) = (
            Rc::clone(&self.chat.resizing),
            Rc::clone(&self.chat.resizing),
            Rc::clone(&self.chat.resizing),
        );
        Some(
            div()
                .id("chat-resize")
                .occlude()
                .absolute()
                .top(px(theme::CHROME_HEIGHT))
                .bottom_0()
                .right(px(model.width - RESIZE_HANDLE / 2.0))
                .w(px(RESIZE_HANDLE))
                .cursor(CursorStyle::ResizeLeftRight)
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    press.set(true);
                    cx.stop_propagation();
                })
                .child(
                    canvas(
                        |_, _, _| (),
                        move |_, (), window, _| {
                            // The drag is the window's until the button comes
                            // up: the pointer leaves the strip at once.
                            window.on_mouse_event(
                                move |event: &MouseMoveEvent, phase, window, cx| {
                                    if phase != DispatchPhase::Capture || !drag.get() {
                                        return;
                                    }
                                    let right = window.viewport_size().width;
                                    let width = f32::from(right - event.position.x);
                                    app_canvas::dispatch(Event::Action(Action::Chat(
                                        ChatAction::Resize(width),
                                    )));
                                    cx.stop_propagation();
                                },
                            );
                            window.on_mouse_event(move |_: &MouseUpEvent, phase, _, _| {
                                if phase == DispatchPhase::Capture {
                                    lift.set(false);
                                }
                            });
                        },
                    )
                    .size_full(),
                ),
        )
    }
}
