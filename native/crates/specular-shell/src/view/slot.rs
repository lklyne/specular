//! The canvas slot: a GPUI element that paints nothing, takes part in
//! layout, hit-testing and focus like any other, and turns what GPUI gives
//! it into the app's [`Event`]s.

use std::cell::Cell;
use std::rc::Rc;

use glam::Vec2;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{
    App, Bounds, Context, CursorStyle, DispatchPhase, ElementInputHandler, Entity, ExternalPaths,
    FocusHandle, Hitbox, HitboxBehavior, InteractiveElement as _, IntoElement, KeyDownEvent,
    KeyUpEvent, Modifiers, ModifiersChangedEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement as _, PinchEvent, Pixels, Point, ScrollDelta, ScrollWheelEvent,
    Styled as _, Window, canvas, div,
};
use specular_core::{PointerButton, PointerEventKind};
use specular_interact::{Cursor, Event, PointerInput, WheelInput};

use super::ShellView;
use crate::canvas;
use crate::keys::{self, RawKind};
use crate::menus::CanvasKey;
use crate::surface::WindowAsks;

/// Logical pixels per wheel "line" for mice that report line deltas.
const PIXELS_PER_LINE: f32 = 40.0;

/// What the slot remembers about the pointer between events.
#[derive(Debug, Default)]
pub(super) struct Pointer {
    /// Whether a press that began on the canvas is still held: its drag and
    /// release belong to the canvas wherever the pointer goes.
    held: Cell<bool>,
    /// Whether the pointer was over the canvas at its last move.
    inside: Cell<bool>,
    /// Whether the press now held only closed a Kit list or menu, so its
    /// release is not the canvas's either.
    swallowed: Cell<bool>,
}

fn modifiers(modifiers: Modifiers) -> specular_core::Modifiers {
    specular_core::Modifiers {
        shift: modifiers.shift,
        control: modifiers.control,
        alt: modifiers.alt,
        meta: modifiers.platform,
    }
}

const fn button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Left),
        MouseButton::Right => Some(PointerButton::Right),
        MouseButton::Middle => Some(PointerButton::Middle),
        MouseButton::Navigate(_) => None,
    }
}

const fn cursor_style(cursor: Cursor) -> CursorStyle {
    match cursor {
        Cursor::Default => CursorStyle::Arrow,
        Cursor::Crosshair => CursorStyle::Crosshair,
        Cursor::Grab => CursorStyle::OpenHand,
        // GPUI has no four-way move cursor.
        Cursor::Grabbing | Cursor::Move => CursorStyle::ClosedHand,
        Cursor::Text => CursorStyle::IBeam,
        Cursor::Pointer => CursorStyle::PointingHand,
        Cursor::ResizeEw => CursorStyle::ResizeLeftRight,
        Cursor::ResizeNs => CursorStyle::ResizeUpDown,
        Cursor::ResizeColumn => CursorStyle::ResizeColumn,
        Cursor::ResizeRow => CursorStyle::ResizeRow,
        Cursor::VerticalText => CursorStyle::IBeamCursorForVerticalLayout,
        Cursor::NotAllowed => CursorStyle::OperationNotAllowed,
        Cursor::Alias => CursorStyle::DragLink,
        Cursor::Copy => CursorStyle::DragCopy,
        Cursor::ContextMenu => CursorStyle::ContextualMenu,
        Cursor::ResizeNwse => CursorStyle::ResizeUpLeftDownRight,
        Cursor::ResizeNesw => CursorStyle::ResizeUpRightDownLeft,
    }
}

/// A window position as the app's screen position: from the slot's corner.
fn screen(position: Point<Pixels>, bounds: Bounds<Pixels>) -> Vec2 {
    Vec2::new(
        f32::from(position.x - bounds.origin.x),
        f32::from(position.y - bounds.origin.y),
    )
}

fn pointer(kind: PointerEventKind, at: Vec2, held: Modifiers) {
    canvas::dispatch(Event::Pointer(PointerInput {
        kind,
        screen: at,
        modifiers: modifiers(held),
    }));
}

/// Sends the key event `AppKit` is dispatching to the app. GPUI's own event
/// has no key code, so the facts come from the monitor's note of it.
pub(crate) fn send_current_key(kind_is: impl Fn(RawKind) -> bool) {
    if let Some(raw) = keys::current().filter(|raw| kind_is(raw.kind)) {
        canvas::dispatch(Event::Key(keys::key_input(&raw)));
    }
}

/// What the slot's paint needs: it registers this frame's handlers.
struct SlotPaint {
    focus: FocusHandle,
    entity: Entity<ShellView>,
    asks: Rc<WindowAsks>,
    pointer: Rc<Pointer>,
    /// Whether the right panel's composer has the keys.
    composing: bool,
}

impl SlotPaint {
    fn paint(&self, bounds: Bounds<Pixels>, hitbox: &Hitbox, window: &mut Window, cx: &mut App) {
        window.set_cursor_style(cursor_style(self.asks.cursor.get()), hitbox);

        // An open Kit list or menu holds the focus, and a press outside it
        // closes it. That press must not also start a gesture on the canvas
        // (the built-in panels swallow it the same way), so a press that
        // arrives while something other than the canvas or a text field has
        // the focus only gives the focus back.
        let closes_a_list = !self.composing
            && window.focused(cx).is_some_and(|focused| {
                focused != self.focus && !super::field::holds_focus(&focused, window, cx)
            });

        let (over, press, grab) = (hitbox.clone(), Rc::clone(&self.pointer), self.focus.clone());
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble || !over.is_hovered(window) {
                return;
            }
            window.focus(&grab, cx);
            if closes_a_list {
                press.swallowed.set(true);
                return;
            }
            let Some(button) = button(event.button) else {
                return;
            };
            press.held.set(true);
            let click_count = u8::try_from(event.click_count).unwrap_or(u8::MAX);
            let kind = PointerEventKind::Down {
                button,
                click_count,
            };
            pointer(kind, screen(event.position, bounds), event.modifiers);
        });

        let (over, press) = (hitbox.clone(), Rc::clone(&self.pointer));
        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, _| {
            if phase != DispatchPhase::Bubble {
                return;
            }
            if press.swallowed.replace(false) {
                return;
            }
            let ours = press.held.replace(false) || over.is_hovered(window);
            let Some(button) = button(event.button).filter(|_| ours) else {
                return;
            };
            let click_count = u8::try_from(event.click_count).unwrap_or(u8::MAX);
            let kind = PointerEventKind::Up {
                button,
                click_count,
            };
            pointer(kind, screen(event.position, bounds), event.modifiers);
        });

        let (over, press) = (hitbox.clone(), Rc::clone(&self.pointer));
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, _| {
            if phase != DispatchPhase::Bubble {
                return;
            }
            let at = screen(event.position, bounds);
            let inside = over.is_hovered(window);
            let was_inside = press.inside.replace(inside);
            if inside || press.held.get() {
                pointer(PointerEventKind::Move, at, event.modifiers);
            } else if was_inside {
                pointer(PointerEventKind::Leave, at, event.modifiers);
            }
        });

        let over = hitbox.clone();
        window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, _| {
            if phase != DispatchPhase::Bubble || !over.is_hovered(window) {
                return;
            }
            let delta = match event.delta {
                ScrollDelta::Pixels(by) => Vec2::new(f32::from(by.x), f32::from(by.y)),
                ScrollDelta::Lines(by) => Vec2::new(by.x, by.y) * PIXELS_PER_LINE,
            };
            canvas::dispatch(Event::Wheel(WheelInput {
                delta,
                modifiers: modifiers(event.modifiers),
            }));
        });

        let over = hitbox.clone();
        window.on_mouse_event(move |event: &PinchEvent, phase, window, _| {
            if phase == DispatchPhase::Bubble && over.is_hovered(window) {
                canvas::dispatch(Event::Pinch { delta: event.delta });
            }
        });

        // The input method composes into the canvas only
        // while a text is being edited or a page is entered.
        if self.asks.ime_allowed.get() {
            let handler = ElementInputHandler::new(bounds, self.entity.clone());
            window.handle_input(&self.focus, handler, cx);
        }
    }
}

impl ShellView {
    /// The slot. Its bounds are the app's viewport.
    pub(super) fn canvas_slot(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl IntoElement {
        let paint = SlotPaint {
            composing: self.composer_focused(window, cx),
            focus: self.focus.clone(),
            entity: cx.entity(),
            asks: Rc::clone(&self.asks),
            pointer: Rc::clone(&self.pointer),
        };
        div()
            .id("canvas-slot")
            .size_full()
            .track_focus(&self.focus)
            .key_context("Canvas")
            // A key that reaches here was not taken by a Kit text field, a
            // shell shortcut or the input method. The app's binding table
            // decides the rest, so GPUI is told the key is spoken for.
            .on_key_down(|_: &KeyDownEvent, _, cx| {
                send_current_key(|kind| matches!(kind, RawKind::Down { .. }));
                cx.stop_propagation();
            })
            .on_key_up(|_: &KeyUpEvent, _, cx| {
                send_current_key(|kind| kind == RawKind::Up);
                cx.stop_propagation();
            })
            .on_modifiers_changed(|_: &ModifiersChangedEvent, _, _| {
                send_current_key(|kind| kind == RawKind::Flags);
            })
            // The Kit's root moves focus on Tab. Over the canvas Tab is a
            // key like any other: it indents a list, and a page wants it.
            .on_action(|_: &CanvasKey, _, _| {
                send_current_key(|kind| matches!(kind, RawKind::Down { .. }));
            })
            .drag_over::<ExternalPaths>(|style, _, _, cx| style.bg(cx.theme().drop_target))
            .on_drop(move |paths: &ExternalPaths, window, cx| {
                let at = window.mouse_position();
                super::drop_files(
                    paths.paths().to_vec(),
                    Vec2::new(f32::from(at.x), f32::from(at.y)),
                );
                cx.stop_propagation();
            })
            .child(
                canvas(
                    move |bounds, window, _| {
                        canvas::with(|canvas| {
                            canvas.set_slot(
                                Vec2::new(f32::from(bounds.origin.x), f32::from(bounds.origin.y)),
                                Vec2::new(
                                    f32::from(bounds.size.width),
                                    f32::from(bounds.size.height),
                                ),
                            );
                        });
                        window.insert_hitbox(bounds, HitboxBehavior::Normal)
                    },
                    move |bounds, hitbox, window, cx| paint.paint(bounds, &hitbox, window, cx),
                )
                .size_full(),
            )
    }
}
