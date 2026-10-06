//! Routing window input: canvas pan/zoom, click-to-focus, and forwarding
//! pointer, wheel, key and IME events to pages.
//!
//! Wheel over the focused page scrolls that page; anywhere else it pans the
//! canvas, and Cmd/Ctrl+wheel or a pinch zooms about the cursor.
//!
//! With the chrome layer on, a press first goes to its gestures (comment
//! tool, resize handles, Alt+drag move); a gesture owns the pointer until the
//! button comes up, and the page under it sees none of it.

use std::time::Instant;

use glam::Vec2;
use specular_core::{
    CssSize, InputEvent, PageId, PointerButton, PointerEvent, PointerEventKind, ViewportInputDelta,
    WheelEvent,
};
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::scancode::PhysicalKeyExtScancode as _;
use winit::window::CursorIcon;

use super::App;
use crate::chrome_state::{ChromeState, Press, Release};
use crate::input_map::{self, KeyPress};
use crate::placement::{PlacedPage, hit_test};

/// Logical pixels per wheel "line" for mice that report line deltas.
const PIXELS_PER_LINE: f32 = 40.0;

impl App {
    pub(super) fn on_input(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self
                    .gpu
                    .as_ref()
                    .map_or(1.0, |gpu| gpu.window.scale_factor());
                let logical = position.to_logical::<f32>(scale);
                self.on_cursor_moved(Vec2::new(logical.x, logical.y));
            }
            WindowEvent::CursorLeft { .. } => {
                // A drag that leaves the window still ends at its release.
                if self.chrome.as_ref().is_some_and(ChromeState::dragging) {
                    return;
                }
                self.cursor = None;
                self.set_hovered(None, None);
            }
            WindowEvent::MouseInput { state, button, .. } => self.on_mouse_button(state, button),
            WindowEvent::MouseWheel { delta, .. } => self.on_wheel(delta),
            WindowEvent::PinchGesture { delta, .. } => self.on_pinch(delta as f32),
            WindowEvent::KeyboardInput { event, .. } => {
                if self.on_chrome_key(&event) {
                    self.show_tool_cursor();
                    return;
                }
                let Some(page) = self.focused else {
                    return;
                };
                let native_key_code = event
                    .physical_key
                    .to_scancode()
                    .map_or(0, |code| code as i32);
                let code = match event.physical_key {
                    PhysicalKey::Code(code) => Some(code),
                    PhysicalKey::Unidentified(_) => None,
                };
                let mut events = std::mem::take(&mut self.key_scratch);
                input_map::key_events(
                    &KeyPress {
                        code,
                        native_key_code,
                        pressed: event.state == ElementState::Pressed,
                        text: event.text.as_deref(),
                        modifiers: input_map::modifiers(self.modifiers),
                    },
                    &mut events,
                );
                for key in events.drain(..) {
                    self.send_to_page(page, &key);
                }
                self.key_scratch = events;
            }
            WindowEvent::Ime(ime) => {
                if let (Some(page), Some(event)) = (self.focused, input_map::ime_event(&ime)) {
                    self.send_to_page(page, &InputEvent::Ime(event));
                }
            }
            _ => {}
        }
    }

    fn placed_page(&self, page: PageId) -> Option<PlacedPage> {
        self.placed
            .iter()
            .find(|placed| placed.page == page)
            .copied()
    }

    fn page_under(&self, screen: Vec2) -> Option<(PlacedPage, Vec2)> {
        let world = self.camera.screen_to_world(screen);
        hit_test(&self.placed, world).map(|placed| (placed, placed.page_local(world)))
    }

    fn pointer(&self, kind: PointerEventKind, position: Vec2) -> InputEvent {
        InputEvent::Pointer(PointerEvent {
            kind,
            position,
            modifiers: input_map::modifiers(self.modifiers),
        })
    }

    fn send_to_page(&mut self, page: PageId, event: &InputEvent) {
        if let Err(error) = self.source.send_input(page, event) {
            tracing::warn!("{error}");
            return;
        }
        let is_hover = matches!(
            event,
            InputEvent::Pointer(PointerEvent {
                kind: PointerEventKind::Move | PointerEventKind::Leave,
                ..
            })
        );
        if !is_hover {
            self.latency.input_sent(page, Instant::now());
        }
    }

    /// `C` toggles the comment tool while no page has keyboard focus (a
    /// focused page gets the key); `Escape` always cancels the drag, leaves
    /// the tool and clears page focus, so it is never forwarded. Returns
    /// whether the chrome layer consumed the key.
    fn on_chrome_key(&mut self, event: &KeyEvent) -> bool {
        if self.chrome.is_none() {
            return false;
        }
        let pressed = event.state == ElementState::Pressed;
        match event.physical_key {
            PhysicalKey::Code(KeyCode::Escape) => {
                if pressed && let Some(chrome) = self.chrome.as_mut() {
                    chrome.escape(&mut self.placed);
                    self.set_focus(None);
                }
                true
            }
            PhysicalKey::Code(KeyCode::KeyC) if pressed && self.modifiers.is_empty() => {
                let focused = self.focused.is_some();
                !event.repeat && self.chrome.as_mut().is_some_and(|c| c.toggle_tool(focused))
            }
            _ => false,
        }
    }

    /// A crosshair while the comment tool is armed.
    fn show_tool_cursor(&self) {
        let armed = self.chrome.as_ref().is_some_and(ChromeState::tool_armed);
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.window.set_cursor(if armed {
                CursorIcon::Crosshair
            } else {
                CursorIcon::Default
            });
        }
    }

    fn on_cursor_moved(&mut self, screen: Vec2) {
        self.cursor = Some(screen);
        if let Some(chrome) = self.chrome.as_mut()
            && chrome.dragging()
        {
            chrome.drag(screen, &self.camera, &mut self.placed);
            return;
        }
        match self.page_under(screen) {
            Some((placed, local)) => {
                self.set_hovered(Some(placed.page), Some(local));
                let event = self.pointer(PointerEventKind::Move, local);
                self.send_to_page(placed.page, &event);
            }
            None => self.set_hovered(None, None),
        }
    }

    /// Tracks the hovered page, sending `Leave` to the one the cursor left.
    fn set_hovered(&mut self, page: Option<PageId>, local: Option<Vec2>) {
        if self.hovered == page {
            return;
        }
        if let Some(previous) = self.hovered {
            let event = self.pointer(PointerEventKind::Leave, local.unwrap_or_default());
            self.send_to_page(previous, &event);
        }
        self.hovered = page;
    }

    fn on_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        let (Some(screen), Some(button)) = (self.cursor, input_map::pointer_button(button)) else {
            return;
        };
        match state {
            ElementState::Pressed => {
                if button == PointerButton::Left
                    && let Some(chrome) = self.chrome.as_mut()
                    && chrome.press(screen, &self.camera, &self.placed, self.modifiers.alt_key())
                        == Press::Consumed
                {
                    return;
                }
                let hit = self.page_under(screen);
                if button == PointerButton::Left {
                    self.set_focus(hit.map(|(placed, _)| placed.page));
                }
                if let Some((placed, local)) = hit {
                    self.captured.press(button, placed.page);
                    let click_count = self.clicks.press(button, screen, Instant::now());
                    let event = self.pointer(
                        PointerEventKind::Down {
                            button,
                            click_count,
                        },
                        local,
                    );
                    self.send_to_page(placed.page, &event);
                }
            }
            ElementState::Released => {
                if button == PointerButton::Left
                    && let Some(chrome) = self.chrome.as_mut()
                {
                    match chrome.release(screen, &self.camera, &self.placed) {
                        Release::NoGesture => {}
                        Release::Finished => return,
                        Release::Resized { page, viewport } => {
                            self.apply_viewport(page, viewport);
                            return;
                        }
                    }
                }
                // The release goes to the page that got the press even if the
                // pointer has left it, so drags that end outside complete and
                // no page is left thinking a button is held.
                let Some(placed) = self
                    .captured
                    .release(button)
                    .and_then(|page| self.placed_page(page))
                else {
                    return;
                };
                let local = placed.page_local(self.camera.screen_to_world(screen));
                let click_count = self.clicks.current();
                let event = self.pointer(
                    PointerEventKind::Up {
                        button,
                        click_count,
                    },
                    local,
                );
                self.send_to_page(placed.page, &event);
            }
        }
    }

    /// Applies a resized page's new layout viewport, once per resize.
    fn apply_viewport(&mut self, page: PageId, viewport: CssSize) {
        if let Err(error) = self.source.set_viewport(page, viewport) {
            tracing::warn!("{error}");
            return;
        }
        if let Some(placed) = self.placed.iter_mut().find(|placed| placed.page == page) {
            placed.viewport = viewport;
        }
    }

    fn set_focus(&mut self, page: Option<PageId>) {
        if self.focused == page {
            return;
        }
        if let Err(error) = self.source.set_focus(page) {
            tracing::warn!("{error}");
            return;
        }
        self.focused = page;
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.window.set_ime_allowed(page.is_some());
        }
    }

    fn zoom_modifier(&self) -> bool {
        self.modifiers.super_key() || self.modifiers.control_key()
    }

    fn on_wheel(&mut self, delta: MouseScrollDelta) {
        let delta = match delta {
            MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y) * PIXELS_PER_LINE,
            MouseScrollDelta::PixelDelta(position) => {
                let scale = self
                    .gpu
                    .as_ref()
                    .map_or(1.0, |gpu| gpu.window.scale_factor());
                let logical = position.to_logical::<f32>(scale);
                Vec2::new(logical.x, logical.y)
            }
        };
        if !self.zoom_modifier()
            && let Some(screen) = self.cursor
            && let Some((placed, local)) = self.page_under(screen)
            && self.focused == Some(placed.page)
        {
            let css_delta = delta / self.camera.zoom / placed.canvas_per_css();
            let event = InputEvent::Wheel(WheelEvent {
                position: local,
                delta: css_delta,
                modifiers: input_map::modifiers(self.modifiers),
            });
            self.send_to_page(placed.page, &event);
            return;
        }
        let input = if self.zoom_modifier() {
            ViewportInputDelta {
                zoom_delta_y: -delta.y,
                anchor: self.cursor,
                ..ViewportInputDelta::default()
            }
        } else {
            ViewportInputDelta {
                pan: delta,
                ..ViewportInputDelta::default()
            }
        };
        self.camera.apply_input_delta(input);
    }

    /// Trackpad pinch: `delta` is the magnification change (positive zooms in).
    fn on_pinch(&mut self, delta: f32) {
        let anchor = self.cursor.unwrap_or_else(|| {
            self.gpu
                .as_ref()
                .map_or(Vec2::ZERO, |gpu| gpu.logical_viewport() / 2.0)
        });
        self.camera
            .zoom_about(anchor, self.camera.zoom * (1.0 + delta));
    }
}
