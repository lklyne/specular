//! The canvas as a Rune element: one window-sized element under the chrome
//! that turns the pointer, the wheel and the trackpad into the app's
//! [`Event`]s, as the Kit's canvas slot (`kit/view/slot.rs`) does.
//!
//! The element is the chrome's sibling and painted first, so a press on a
//! toolbar control is the control's alone. A press, its drag and its release
//! come through `on_drag`, which Rune hands the whole gesture, with the run
//! of clicks; a move with no button held through `on_pointer_move_local`.
//! Positions are window points, which are the canvas's: the element fills
//! the window from its corner.

use glam::Vec2;
use rune::{
    Cursor, Drag, El, MagnifyEvent, Phase, PointerButton as RuneButton, PointerEvent, Tx,
    WheelEvent, el,
};
use specular_core::{PointerButton, PointerEventKind};
use specular_interact::{Effect, Event, Key, KeyInput, PhysicalKey, PointerInput, WheelInput};

use crate::keys;
use crate::shell::Shell;

/// What effects asked of the window: the cursor over the canvas and the
/// input method. The view reads them.
#[derive(Debug, Clone, Default)]
pub struct Asks {
    /// The cursor the app shows over the canvas.
    pub cursor: specular_interact::Cursor,
    /// Whether the input method composes into the canvas.
    pub ime_allowed: bool,
    /// Where the caret is, for the candidate window: origin and size.
    pub ime_area: (Vec2, Vec2),
}

impl Asks {
    /// Takes `effect` if it is one of these. Returns whether it was.
    pub fn note(&mut self, effect: &Effect) -> bool {
        match effect {
            Effect::SetCursor(cursor) => self.cursor = *cursor,
            Effect::SetImeAllowed(allowed) => self.ime_allowed = *allowed,
            Effect::SetImeCursorArea { origin, size } => self.ime_area = (*origin, *size),
            _ => return false,
        }
        true
    }
}

/// The app's cursor as Rune's. Rune has no vertical I-beam, alias, copy or
/// context-menu cursor, and the app's own arrows are the system's here.
fn cursor(cursor: specular_interact::Cursor) -> Cursor {
    use specular_interact::Cursor as C;
    match cursor {
        C::Default | C::PageArrow | C::Touch | C::Copy | C::ContextMenu | C::Alias => {
            Cursor::Default
        }
        C::Crosshair => Cursor::Crosshair,
        C::Grab => Cursor::Grab,
        C::Grabbing => Cursor::Grabbing,
        C::Move => Cursor::Move,
        C::Text | C::VerticalText => Cursor::Text,
        C::Pointer => Cursor::Pointer,
        C::ResizeEw => Cursor::ResizeEw,
        C::ResizeNs => Cursor::ResizeNs,
        C::ResizeColumn => Cursor::ResizeColumn,
        C::ResizeRow => Cursor::ResizeRow,
        C::NotAllowed => Cursor::NotAllowed,
        C::ResizeNwse => Cursor::ResizeNwse,
        C::ResizeNesw => Cursor::ResizeNesw,
    }
}

fn button(button: RuneButton) -> Option<PointerButton> {
    match button {
        RuneButton::Primary => Some(PointerButton::Left),
        RuneButton::Secondary => Some(PointerButton::Right),
        RuneButton::Middle => Some(PointerButton::Middle),
        _ => None,
    }
}

fn pointer(shell: &mut Shell, kind: PointerEventKind, [x, y]: [f32; 2], tx: &Tx) {
    shell.dispatch(Event::Pointer(PointerInput {
        kind,
        screen: Vec2::new(x, y),
        modifiers: keys::modifiers(tx.mods()),
    }));
}

/// Escape, as the Kit would have sent it mid-drag. Rune keeps an Escape
/// pressed during a held drag for itself and cancels the drag.
fn escape() -> Option<KeyInput> {
    let key = PhysicalKey::for_key(Key::Escape)?;
    let modifiers = specular_core::Modifiers::default();
    Some(specular_interact::mac_key_input(
        key.mac_key_code(),
        true,
        false,
        "",
        modifiers,
    ))
}

/// A press, its drag and its release.
fn on_drag(shell: &mut Shell, drag: &mut Drag, tx: &mut Tx) {
    let Some(button) = button(tx.button()) else {
        return;
    };
    let click_count = u8::try_from(drag.clicks).unwrap_or(u8::MAX);
    let at = drag.window_point();
    let kind = match drag.phase {
        Phase::Down => {
            // A press on the canvas closes the toolbar's open list, as a
            // press outside a Kit list does.
            shell.open_list = None;
            PointerEventKind::Down {
                button,
                click_count,
            }
        }
        Phase::Move => PointerEventKind::Move,
        Phase::Up => {
            if drag.cancelled()
                && let Some(escape) = escape()
            {
                shell.dispatch(Event::Key(escape));
            }
            PointerEventKind::Up {
                button,
                click_count,
            }
        }
    };
    pointer(shell, kind, at, tx);
}

/// A move with no button held: the hover.
fn on_move(shell: &mut Shell, event: PointerEvent, tx: &mut Tx) {
    if !event.pressing {
        let at = [event.window_x, event.window_y];
        pointer(shell, PointerEventKind::Move, at, tx);
    }
}

fn on_hover_change(shell: &mut Shell, inside: bool, tx: &mut Tx) {
    if !inside {
        // A leave's position is unused.
        pointer(shell, PointerEventKind::Leave, [0.0, 0.0], tx);
    }
}

/// A wheel or two-finger scroll: Rune's deltas move content right and down
/// when positive, as the app's do. With Command held the app zooms.
fn on_wheel(shell: &mut Shell, event: WheelEvent, _: &mut Tx) -> bool {
    if event.dx != 0.0 || event.dy != 0.0 {
        shell.dispatch(Event::Wheel(WheelInput {
            delta: Vec2::new(event.dx, event.dy),
            modifiers: keys::modifiers(event.mods),
        }));
    }
    true
}

/// A trackpad pinch, or Control and the wheel, which Rune turns into one.
fn on_magnify(shell: &mut Shell, event: MagnifyEvent, _: &mut Tx) -> bool {
    shell.dispatch(Event::Pinch { delta: event.delta });
    true
}

/// The canvas element, the size of the window. While the app has the input
/// method on, a caret-sized child marks where the candidate window goes.
fn canvas(shell: &Shell) -> El {
    let mut layer = el()
        .key("canvas")
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .cursor(cursor(shell.asks.cursor))
        .pointer_buttons([RuneButton::Secondary, RuneButton::Middle])
        .on_drag(on_drag)
        .on_pointer_move_local(on_move)
        .on_hover_change(on_hover_change)
        .on_wheel(on_wheel)
        .on_magnify(on_magnify);
    if shell.asks.ime_allowed {
        let (origin, size) = shell.asks.ime_area;
        layer = layer.child(
            el().absolute(origin.x, origin.y)
                .size(size.x.max(1.0), size.y.max(1.0))
                .ime_cursor(),
        );
    }
    layer
}

/// The whole window: the canvas, and `chrome` over it. The root hears every
/// key release nothing else took.
pub fn window(shell: &Shell, chrome: El) -> El {
    el().w_full()
        .h_full()
        .on_key_up(keys::on_key_up)
        .child(canvas(shell))
        .child(el().absolute(0.0, 0.0).w_full().h_full().child(chrome))
}
