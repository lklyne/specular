//! The strip along the right panel's left edge that resizes it.

use std::rc::Rc;

use gpui_kit::{
    CursorStyle, DispatchPhase, InteractiveElement as _, IntoElement, MouseButton, MouseMoveEvent,
    MouseUpEvent, ParentElement as _, Styled as _, canvas, div, px,
};
use specular_interact::{Action, ChatAction, Event};

use super::ShellView;
use crate::canvas as app_canvas;
use crate::theme;

/// The strip's width, `DEVTOOLS_RESIZE_HANDLE_WIDTH`.
const RESIZE_HANDLE: f32 = 12.0;

impl ShellView {
    /// The strip over the edge of a panel `width` wide. Dragging it asks
    /// for the width that puts the edge under the pointer, and the model
    /// keeps that within its limits.
    pub(super) fn chat_resize_handle(&self, width: f32) -> impl IntoElement + use<> {
        let (press, drag, lift) = (
            Rc::clone(&self.chat_resizing),
            Rc::clone(&self.chat_resizing),
            Rc::clone(&self.chat_resizing),
        );
        div()
            .id("chat-resize")
            .occlude()
            .absolute()
            .top(px(theme::TOOLBAR_HEIGHT))
            .bottom_0()
            .right(px(width - RESIZE_HANDLE / 2.0))
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
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                            if phase != DispatchPhase::Capture || !drag.get() {
                                return;
                            }
                            let right = window.viewport_size().width;
                            let width = f32::from(right - event.position.x);
                            app_canvas::dispatch(Event::Action(Action::Chat(ChatAction::Resize(
                                width,
                            ))));
                            cx.stop_propagation();
                        });
                        window.on_mouse_event(move |_: &MouseUpEvent, phase, _, _| {
                            if phase == DispatchPhase::Capture {
                                lift.set(false);
                            }
                        });
                    },
                )
                .size_full(),
            )
    }
}
