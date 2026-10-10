//! Translates winit input into [`Event`](specular_interact::Event) payloads.

use std::time::{Duration, Instant};

use glam::Vec2;
use specular_core::{ImeEvent, Modifiers, PointerButton};
use specular_interact::{Appearance, Cursor, Key, KeyInput};
use winit::event::{ElementState, Ime, KeyEvent, MouseButton};
use winit::keyboard::ModifiersState;
use winit::platform::scancode::PhysicalKeyExtScancode as _;
use winit::window::{CursorIcon, Theme};

/// Presses closer together than this (and [`CLICK_SLOP`]) extend a click run.
const MULTI_CLICK_INTERVAL: Duration = Duration::from_millis(500);
/// Maximum pointer travel, in logical pixels, between presses of a run.
const CLICK_SLOP: f32 = 4.0;

/// winit modifier state as page modifiers.
pub(crate) fn modifiers(state: ModifiersState) -> Modifiers {
    Modifiers {
        shift: state.shift_key(),
        control: state.control_key(),
        alt: state.alt_key(),
        meta: state.super_key(),
    }
}

/// The page button for a winit mouse button; extra buttons are not forwarded.
pub(crate) fn pointer_button(button: MouseButton) -> Option<PointerButton> {
    match button {
        MouseButton::Left => Some(PointerButton::Left),
        MouseButton::Middle => Some(PointerButton::Middle),
        MouseButton::Right => Some(PointerButton::Right),
        MouseButton::Back | MouseButton::Forward | MouseButton::Other(_) => None,
    }
}

/// The appearance the window system reports.
pub(crate) const fn appearance(theme: Theme) -> Appearance {
    match theme {
        Theme::Light => Appearance::Light,
        Theme::Dark => Appearance::Dark,
    }
}

/// The winit icon for a cursor.
pub(crate) fn cursor_icon(cursor: Cursor) -> CursorIcon {
    match cursor {
        Cursor::Default => CursorIcon::Default,
        Cursor::Crosshair => CursorIcon::Crosshair,
        Cursor::Grab => CursorIcon::Grab,
        Cursor::Grabbing => CursorIcon::Grabbing,
        Cursor::Move => CursorIcon::Move,
        Cursor::Text => CursorIcon::Text,
        Cursor::Pointer => CursorIcon::Pointer,
        Cursor::ResizeEw => CursorIcon::EwResize,
        Cursor::ResizeNs => CursorIcon::NsResize,
        Cursor::ResizeColumn => CursorIcon::ColResize,
        Cursor::ResizeRow => CursorIcon::RowResize,
        Cursor::VerticalText => CursorIcon::VerticalText,
        Cursor::NotAllowed => CursorIcon::NotAllowed,
        Cursor::Alias => CursorIcon::Alias,
        Cursor::Copy => CursorIcon::Copy,
        Cursor::ContextMenu => CursorIcon::ContextMenu,
        Cursor::ResizeNwse => CursorIcon::NwseResize,
        Cursor::ResizeNesw => CursorIcon::NeswResize,
    }
}

/// Counts consecutive presses for `click_count` (double/triple click).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ClickCounter {
    last: Option<(Instant, Vec2, PointerButton)>,
    count: u8,
}

impl ClickCounter {
    /// Registers a press at `position` (screen logical px); returns its
    /// click count, starting at 1.
    pub(crate) fn press(&mut self, button: PointerButton, position: Vec2, now: Instant) -> u8 {
        let continues = self.last.is_some_and(|(at, from, previous)| {
            previous == button
                && now.saturating_duration_since(at) <= MULTI_CLICK_INTERVAL
                && from.distance(position) <= CLICK_SLOP
        });
        self.count = if continues {
            self.count.saturating_add(1)
        } else {
            1
        };
        self.last = Some((now, position, button));
        self.count
    }

    /// The count of the most recent press, for its release.
    pub(crate) fn current(&self) -> u8 {
        self.count.max(1)
    }
}

/// One winit key transition as a [`KeyInput`]. winit's scancode is the
/// platform's own key code (the `kVK_*` code on macOS), which is what the
/// key tables read. A release has no text in winit, so its characters are
/// the logical key's.
pub(crate) fn key_input(event: &KeyEvent, modifiers: Modifiers) -> KeyInput {
    let pressed = event.state == ElementState::Pressed;
    let characters = match (&event.text, &event.logical_key) {
        (Some(text), _) | (None, winit::keyboard::Key::Character(text)) => text.as_str(),
        (None, _) => "",
    };
    let code = (event.physical_key.to_scancode()).and_then(|code| u16::try_from(code).ok());
    match code {
        Some(code) => {
            specular_interact::mac_key_input(code, pressed, event.repeat, characters, modifiers)
        }
        None => KeyInput {
            key: Key::Other,
            pressed,
            repeat: event.repeat,
            text: event.text.as_ref().map(ToString::to_string),
            character: None,
            modifiers,
            windows_key_code: 0,
            native_key_code: 0,
            commands: Vec::new(),
        },
    }
}

/// The page IME event for a winit IME event; `Enabled`/`Disabled` map to
/// nothing.
pub(crate) fn ime_event(event: &Ime) -> Option<ImeEvent> {
    match event {
        Ime::Preedit(text, _) if text.is_empty() => Some(ImeEvent::Cancel),
        Ime::Preedit(text, cursor) => {
            let end = utf16_len(text);
            let selection = cursor.map_or(end..end, |(start, stop)| {
                utf16_offset(text, start)..utf16_offset(text, stop)
            });
            Some(ImeEvent::SetComposition {
                text: text.clone(),
                selection,
                replacement: None,
            })
        }
        Ime::Commit(text) => Some(ImeEvent::Commit {
            text: text.clone(),
            replacement: None,
        }),
        Ime::Enabled | Ime::Disabled => None,
    }
}

fn utf16_len(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

/// UTF-16 offset of UTF-8 byte offset `byte` (clamped to a char boundary).
fn utf16_offset(text: &str, byte: usize) -> u32 {
    let mut end = byte.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    utf16_len(&text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ime_events() {
        // "日本" is 6 UTF-8 bytes and 2 UTF-16 units.
        let event = ime_event(&Ime::Preedit("日本".to_owned(), Some((3, 6))));
        assert!(matches!(
            event,
            Some(ImeEvent::SetComposition { selection, .. }) if selection == (1..2)
        ));
        let event = ime_event(&Ime::Preedit("😀".to_owned(), None));
        assert!(matches!(
            event,
            Some(ImeEvent::SetComposition { selection, .. }) if selection == (2..2)
        ));
        assert_eq!(
            ime_event(&Ime::Preedit(String::new(), None)),
            Some(ImeEvent::Cancel)
        );
        assert_eq!(
            ime_event(&Ime::Commit("ok".to_owned())),
            Some(ImeEvent::Commit {
                text: "ok".to_owned(),
                replacement: None
            })
        );
    }

    #[test]
    fn a_quick_near_press_is_a_double_click() {
        let start = Instant::now();
        let press = |first: Vec2, second: Vec2, gap: Duration| {
            let mut counter = ClickCounter::default();
            counter.press(PointerButton::Left, first, start);
            counter.press(PointerButton::Left, second, start + gap)
        };
        let quick = Duration::from_millis(200);
        assert_eq!(press(Vec2::ZERO, Vec2::new(1.0, 1.0), quick), 2);
        assert_eq!(press(Vec2::ZERO, Vec2::ZERO, Duration::from_secs(1)), 1);
        assert_eq!(press(Vec2::ZERO, Vec2::new(30.0, 0.0), Duration::ZERO), 1);
        // The edges of the run are inside it.
        assert_eq!(press(Vec2::ZERO, Vec2::new(CLICK_SLOP, 0.0), quick), 2);
        assert_eq!(press(Vec2::ZERO, Vec2::ZERO, MULTI_CLICK_INTERVAL), 2);

        let mut counter = ClickCounter::default();
        let counts = [
            counter.press(PointerButton::Left, Vec2::ZERO, start),
            counter.press(PointerButton::Left, Vec2::ZERO, start + quick),
            counter.press(PointerButton::Left, Vec2::ZERO, start + quick * 2),
            counter.press(PointerButton::Right, Vec2::ZERO, start + quick * 3),
        ];
        assert_eq!(counts, [1, 2, 3, 1], "a different button starts over");
    }
}
