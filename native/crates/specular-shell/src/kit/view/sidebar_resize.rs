//! The strip over the sidebar's right edge. Dragging it asks for the width
//! that puts the edge under the pointer, and the model keeps that within
//! its limits. It is the window's, over whatever draws the sidebar, the
//! app's own panel or a mod.

use std::cell::Cell;

use gpui_kit::{
    CursorStyle, DispatchPhase, InteractiveElement as _, IntoElement, MouseButton, MouseMoveEvent,
    MouseUpEvent, ParentElement as _, Styled as _, canvas, div, px,
};
use specular_interact::{Action, Event, SidebarAction, SidebarModel};

use crate::canvas as app_canvas;

/// How wide the strip is, centred on the edge.
const HANDLE: f32 = 8.0;

thread_local! {
    /// Whether a drag that began on the strip is still held.
    static HELD: Cell<bool> = const { Cell::new(false) };
}

/// The strip for `model`'s sidebar, while it is drawn.
pub(super) fn handle(model: &SidebarModel) -> Option<impl IntoElement + use<>> {
    let [x, top, width] = model.edge.filter(|_| model.visible)?;
    Some(
        div()
            .id("sidebar-resize")
            .occlude()
            .absolute()
            .top(px(top))
            .bottom_0()
            .left(px(x + width - HANDLE / 2.0))
            .w(px(HANDLE))
            .cursor(CursorStyle::ResizeLeftRight)
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                HELD.with(|held| held.set(true));
                cx.stop_propagation();
            })
            .child(
                canvas(
                    |_, _, _| (),
                    move |_, (), window, _| {
                        // The drag is the window's until the button comes up:
                        // the pointer leaves the strip at once.
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                            if phase != DispatchPhase::Capture || !HELD.with(Cell::get) {
                                return;
                            }
                            let width = f32::from(event.position.x) - x;
                            app_canvas::dispatch(Event::Action(Action::Sidebar(
                                SidebarAction::Resize(width),
                            )));
                            window.refresh();
                            cx.stop_propagation();
                        });
                        window.on_mouse_event(|_: &MouseUpEvent, phase, _, _| {
                            if phase == DispatchPhase::Capture {
                                HELD.with(|held| held.set(false));
                            }
                        });
                    },
                )
                .size_full(),
            ),
    )
}
