//! Window input as [`Event`]s. Nothing is decided here: what a press, a key
//! or a scroll does is `specular-interact`'s business.

use std::time::Instant;

use glam::Vec2;
use specular_core::PointerEventKind;
use specular_interact::{Event, PointerInput, WheelInput};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

use super::Shell;
use crate::translate;

/// Logical pixels per wheel "line" for mice that report line deltas.
const PIXELS_PER_LINE: f32 = 40.0;

impl Shell {
    pub(super) fn on_input(&mut self, event: WindowEvent) {
        if self.runtime.closing {
            return;
        }
        let modifiers = translate::modifiers(self.modifiers);
        if matches!(
            event,
            WindowEvent::CursorMoved { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::PinchGesture { .. }
                | WindowEvent::KeyboardInput { .. }
        ) {
            self.runtime.input();
        }
        match event {
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::CursorMoved { position, .. } => {
                let logical = position.to_logical::<f32>(self.window_scale());
                let screen = Vec2::new(logical.x, logical.y);
                self.cursor = Some(screen);
                self.pointer(PointerEventKind::Move, screen);
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer(PointerEventKind::Leave, self.cursor.unwrap_or_default());
            }
            WindowEvent::MouseInput { state, button, .. } => self.on_mouse_button(state, button),
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y) * PIXELS_PER_LINE,
                    MouseScrollDelta::PixelDelta(position) => {
                        let logical = position.to_logical::<f32>(self.window_scale());
                        Vec2::new(logical.x, logical.y)
                    }
                };
                self.runtime
                    .dispatch(Event::Wheel(WheelInput { delta, modifiers }));
            }
            WindowEvent::PinchGesture { delta, .. } => self.runtime.dispatch(Event::Pinch {
                delta: delta as f32,
            }),
            WindowEvent::KeyboardInput { event, .. } => {
                self.runtime
                    .dispatch(Event::Key(translate::key_input(&event, modifiers)));
            }
            WindowEvent::Ime(ime) => {
                if let Some(event) = translate::ime_event(&ime) {
                    self.runtime.dispatch(Event::Ime(event));
                }
            }
            _ => {}
        }
    }

    fn window_scale(&self) -> f64 {
        (self.runtime.gpu)
            .as_ref()
            .map_or(1.0, |gpu| gpu.window.scale_factor())
    }

    fn pointer(&mut self, kind: PointerEventKind, screen: Vec2) {
        self.runtime.dispatch(Event::Pointer(PointerInput {
            kind,
            screen,
            modifiers: translate::modifiers(self.modifiers),
        }));
    }

    /// winit reports a button without a position, so it happens where the
    /// pointer last was.
    fn on_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        let (Some(screen), Some(button)) = (self.cursor, translate::pointer_button(button)) else {
            return;
        };
        let kind = match state {
            ElementState::Pressed => PointerEventKind::Down {
                button,
                click_count: self.clicks.press(button, screen, Instant::now()),
            },
            ElementState::Released => PointerEventKind::Up {
                button,
                click_count: self.clicks.current(),
            },
        };
        self.pointer(kind, screen);
    }
}
