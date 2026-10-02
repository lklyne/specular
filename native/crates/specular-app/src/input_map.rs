//! Translates winit input into the page input contract (CEF field layout).

use std::time::{Duration, Instant};

use glam::Vec2;
use specular_core::{
    ImeEvent, InputEvent, KeyEvent, KeyEventKind, Modifiers, PageId, PointerButton,
};
use winit::event::{Ime, MouseButton};
use winit::keyboard::{KeyCode, ModifiersState};

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

/// Which page received each held button's press, so its release goes to the
/// same page (pointer capture per button) wherever the pointer ends up.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ButtonCapture {
    pages: [Option<PageId>; 3],
}

impl ButtonCapture {
    fn slot(button: PointerButton) -> usize {
        match button {
            PointerButton::Left => 0,
            PointerButton::Middle => 1,
            PointerButton::Right => 2,
        }
    }

    /// Records that `page` received `button`'s press.
    pub(crate) fn press(&mut self, button: PointerButton, page: PageId) {
        self.pages[Self::slot(button)] = Some(page);
    }

    /// The page owed `button`'s release, if a page received its press.
    pub(crate) fn release(&mut self, button: PointerButton) -> Option<PageId> {
        self.pages[Self::slot(button)].take()
    }
}

/// Windows virtual-key codes (`WinUser.h`), which Chromium derives DOM
/// `keyCode` from on every platform. `native_key_code` comes from winit's
/// scancode instead (the `kVK_*` code on macOS), from which Chromium derives
/// DOM `code` and key location.
const WINDOWS_KEY_CODES: &[(KeyCode, i32)] = &[
    (KeyCode::Backspace, 0x08),
    (KeyCode::Tab, 0x09),
    (KeyCode::Enter, 0x0D),
    (KeyCode::NumpadEnter, 0x0D),
    (KeyCode::ShiftLeft, 0x10),
    (KeyCode::ShiftRight, 0x10),
    (KeyCode::ControlLeft, 0x11),
    (KeyCode::ControlRight, 0x11),
    (KeyCode::AltLeft, 0x12),
    (KeyCode::AltRight, 0x12),
    (KeyCode::CapsLock, 0x14),
    (KeyCode::Escape, 0x1B),
    (KeyCode::Space, 0x20),
    (KeyCode::PageUp, 0x21),
    (KeyCode::PageDown, 0x22),
    (KeyCode::End, 0x23),
    (KeyCode::Home, 0x24),
    (KeyCode::ArrowLeft, 0x25),
    (KeyCode::ArrowUp, 0x26),
    (KeyCode::ArrowRight, 0x27),
    (KeyCode::ArrowDown, 0x28),
    (KeyCode::Insert, 0x2D),
    (KeyCode::Delete, 0x2E),
    (KeyCode::Digit0, 0x30),
    (KeyCode::Digit1, 0x31),
    (KeyCode::Digit2, 0x32),
    (KeyCode::Digit3, 0x33),
    (KeyCode::Digit4, 0x34),
    (KeyCode::Digit5, 0x35),
    (KeyCode::Digit6, 0x36),
    (KeyCode::Digit7, 0x37),
    (KeyCode::Digit8, 0x38),
    (KeyCode::Digit9, 0x39),
    (KeyCode::KeyA, 0x41),
    (KeyCode::KeyB, 0x42),
    (KeyCode::KeyC, 0x43),
    (KeyCode::KeyD, 0x44),
    (KeyCode::KeyE, 0x45),
    (KeyCode::KeyF, 0x46),
    (KeyCode::KeyG, 0x47),
    (KeyCode::KeyH, 0x48),
    (KeyCode::KeyI, 0x49),
    (KeyCode::KeyJ, 0x4A),
    (KeyCode::KeyK, 0x4B),
    (KeyCode::KeyL, 0x4C),
    (KeyCode::KeyM, 0x4D),
    (KeyCode::KeyN, 0x4E),
    (KeyCode::KeyO, 0x4F),
    (KeyCode::KeyP, 0x50),
    (KeyCode::KeyQ, 0x51),
    (KeyCode::KeyR, 0x52),
    (KeyCode::KeyS, 0x53),
    (KeyCode::KeyT, 0x54),
    (KeyCode::KeyU, 0x55),
    (KeyCode::KeyV, 0x56),
    (KeyCode::KeyW, 0x57),
    (KeyCode::KeyX, 0x58),
    (KeyCode::KeyY, 0x59),
    (KeyCode::KeyZ, 0x5A),
    (KeyCode::SuperLeft, 0x5B),
    (KeyCode::SuperRight, 0x5C),
    (KeyCode::ContextMenu, 0x5D),
    (KeyCode::F1, 0x70),
    (KeyCode::F2, 0x71),
    (KeyCode::F3, 0x72),
    (KeyCode::F4, 0x73),
    (KeyCode::F5, 0x74),
    (KeyCode::F6, 0x75),
    (KeyCode::F7, 0x76),
    (KeyCode::F8, 0x77),
    (KeyCode::F9, 0x78),
    (KeyCode::F10, 0x79),
    (KeyCode::F11, 0x7A),
    (KeyCode::F12, 0x7B),
    (KeyCode::Semicolon, 0xBA),
    (KeyCode::Equal, 0xBB),
    (KeyCode::Comma, 0xBC),
    (KeyCode::Minus, 0xBD),
    (KeyCode::Period, 0xBE),
    (KeyCode::Slash, 0xBF),
    (KeyCode::Backquote, 0xC0),
    (KeyCode::BracketLeft, 0xDB),
    (KeyCode::Backslash, 0xDC),
    (KeyCode::BracketRight, 0xDD),
    (KeyCode::Quote, 0xDE),
];

/// Windows virtual-key code for `code`, or 0 when unmapped.
pub(crate) fn windows_key_code(code: KeyCode) -> i32 {
    WINDOWS_KEY_CODES
        .iter()
        .find(|(known, _)| *known == code)
        .map_or(0, |&(_, vk)| vk)
}

/// One physical key transition, already extracted from winit's `KeyEvent`
/// (which tests cannot construct).
#[derive(Debug, Clone, Copy)]
pub(crate) struct KeyPress<'a> {
    pub(crate) code: Option<KeyCode>,
    pub(crate) native_key_code: i32,
    pub(crate) pressed: bool,
    pub(crate) text: Option<&'a str>,
    pub(crate) modifiers: Modifiers,
}

/// Appends the CEF key sequence for `press`: `RawDown` + one `Char` per
/// produced character on press, `Up` on release. Command-key chords produce
/// no `Char`, as on macOS where they are shortcuts rather than text.
pub(crate) fn key_events(press: &KeyPress<'_>, out: &mut Vec<InputEvent>) {
    let windows_key_code = press.code.map_or(0, windows_key_code);
    let key = |kind, character| {
        InputEvent::Key(KeyEvent {
            kind,
            windows_key_code,
            native_key_code: press.native_key_code,
            character,
            modifiers: press.modifiers,
        })
    };
    if !press.pressed {
        out.push(key(KeyEventKind::Up, None));
        return;
    }
    out.push(key(KeyEventKind::RawDown, None));
    if press.modifiers.meta {
        return;
    }
    for character in press.text.unwrap_or_default().chars() {
        out.push(InputEvent::Key(KeyEvent {
            kind: KeyEventKind::Char,
            windows_key_code: character as i32,
            native_key_code: press.native_key_code,
            character: Some(character),
            modifiers: press.modifiers,
        }));
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

    fn press(code: KeyCode, text: Option<&str>, modifiers: Modifiers) -> Vec<InputEvent> {
        let mut out = Vec::new();
        key_events(
            &KeyPress {
                code: Some(code),
                native_key_code: 0,
                pressed: true,
                text,
                modifiers,
            },
            &mut out,
        );
        out
    }

    fn kinds(events: &[InputEvent]) -> Vec<KeyEventKind> {
        events
            .iter()
            .filter_map(|event| match event {
                InputEvent::Key(key) => Some(key.kind),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn letter_press_sends_raw_down_then_char() {
        let events = press(KeyCode::KeyA, Some("a"), Modifiers::default());
        assert_eq!(kinds(&events), [KeyEventKind::RawDown, KeyEventKind::Char]);
    }

    #[test]
    fn command_chord_sends_no_char() {
        let meta = Modifiers {
            meta: true,
            ..Modifiers::default()
        };
        assert_eq!(
            kinds(&press(KeyCode::KeyC, Some("c"), meta)),
            [KeyEventKind::RawDown]
        );
    }

    #[test]
    fn release_sends_single_up() {
        let mut out = Vec::new();
        key_events(
            &KeyPress {
                code: Some(KeyCode::KeyA),
                native_key_code: 0,
                pressed: false,
                text: None,
                modifiers: Modifiers::default(),
            },
            &mut out,
        );
        assert_eq!(kinds(&out), [KeyEventKind::Up]);
    }

    #[test]
    fn letters_map_to_uppercase_virtual_keys() {
        assert_eq!(windows_key_code(KeyCode::KeyQ), i32::from(b'Q'));
    }

    #[test]
    fn unmapped_key_code_is_zero() {
        assert_eq!(windows_key_code(KeyCode::MediaPlayPause), 0);
    }

    #[test]
    fn preedit_cursor_converts_bytes_to_utf16_units() {
        // "日本" is 6 UTF-8 bytes and 2 UTF-16 units.
        let event = ime_event(&Ime::Preedit("日本".to_owned(), Some((3, 6))));
        assert!(matches!(
            event,
            Some(ImeEvent::SetComposition { selection, .. }) if selection == (1..2)
        ));
    }

    #[test]
    fn preedit_without_cursor_places_caret_at_end() {
        let event = ime_event(&Ime::Preedit("😀".to_owned(), None));
        assert!(matches!(
            event,
            Some(ImeEvent::SetComposition { selection, .. }) if selection == (2..2)
        ));
    }

    #[test]
    fn empty_preedit_cancels_composition() {
        assert_eq!(
            ime_event(&Ime::Preedit(String::new(), None)),
            Some(ImeEvent::Cancel)
        );
    }

    #[test]
    fn commit_maps_to_commit() {
        assert_eq!(
            ime_event(&Ime::Commit("ok".to_owned())),
            Some(ImeEvent::Commit {
                text: "ok".to_owned(),
                replacement: None
            })
        );
    }

    #[test]
    fn quick_second_press_is_a_double_click() {
        let start = Instant::now();
        let mut counter = ClickCounter::default();
        counter.press(PointerButton::Left, Vec2::ZERO, start);
        let count = counter.press(
            PointerButton::Left,
            Vec2::new(1.0, 1.0),
            start + Duration::from_millis(200),
        );
        assert_eq!(count, 2);
    }

    #[test]
    fn slow_second_press_restarts_click_count() {
        let start = Instant::now();
        let mut counter = ClickCounter::default();
        counter.press(PointerButton::Left, Vec2::ZERO, start);
        let count = counter.press(
            PointerButton::Left,
            Vec2::ZERO,
            start + Duration::from_secs(1),
        );
        assert_eq!(count, 1);
    }

    #[test]
    fn distant_second_press_restarts_click_count() {
        let start = Instant::now();
        let mut counter = ClickCounter::default();
        counter.press(PointerButton::Left, Vec2::ZERO, start);
        let count = counter.press(PointerButton::Left, Vec2::new(30.0, 0.0), start);
        assert_eq!(count, 1);
    }

    #[test]
    fn release_goes_to_the_page_that_got_the_press() {
        let mut capture = ButtonCapture::default();
        capture.press(PointerButton::Right, PageId(2));
        capture.press(PointerButton::Left, PageId(1));
        assert_eq!(capture.release(PointerButton::Right), Some(PageId(2)));
    }

    #[test]
    fn release_without_press_goes_nowhere() {
        let mut capture = ButtonCapture::default();
        capture.press(PointerButton::Left, PageId(1));
        capture.release(PointerButton::Left);
        assert_eq!(capture.release(PointerButton::Left), None);
    }

    #[test]
    fn super_maps_to_meta_modifier() {
        assert!(modifiers(ModifiersState::SUPER).meta);
    }
}
