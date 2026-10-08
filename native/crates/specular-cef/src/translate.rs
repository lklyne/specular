//! Translation of [`InputEvent`]s into the exact CEF `BrowserHost` calls.
//!
//! The CEF backend's `send_input` is a field-for-field copy of a
//! [`HostCall`]; every decision (flag bits, rounding, held buttons, wheel
//! remainders, UTF-16 splitting, IME range sentinels) lives here, where it is
//! unit-tested without CEF.

use glam::Vec2;
use specular_core::{
    ImeEvent, InputEvent, KeyEvent, KeyEventKind, Modifiers, PointerButton, PointerEvent,
    PointerEventKind, WheelEvent,
};

/// CEF `cef_event_flags_t` bits (`include/internal/cef_types.h`).
pub mod flags {
    /// `EVENTFLAG_SHIFT_DOWN`.
    pub const SHIFT_DOWN: u32 = 1 << 1;
    /// `EVENTFLAG_CONTROL_DOWN`.
    pub const CONTROL_DOWN: u32 = 1 << 2;
    /// `EVENTFLAG_ALT_DOWN`.
    pub const ALT_DOWN: u32 = 1 << 3;
    /// `EVENTFLAG_LEFT_MOUSE_BUTTON`.
    pub const LEFT_MOUSE_BUTTON: u32 = 1 << 4;
    /// `EVENTFLAG_MIDDLE_MOUSE_BUTTON`.
    pub const MIDDLE_MOUSE_BUTTON: u32 = 1 << 5;
    /// `EVENTFLAG_RIGHT_MOUSE_BUTTON`.
    pub const RIGHT_MOUSE_BUTTON: u32 = 1 << 6;
    /// `EVENTFLAG_COMMAND_DOWN` (macOS Command).
    pub const COMMAND_DOWN: u32 = 1 << 7;
}

/// CEF's `CefRange::InvalidRange()` component, meaning "no range".
pub const INVALID_RANGE_BOUND: u32 = u32::MAX;

/// A UTF-16 range as CEF's `cef_range_t` (`from`, `to`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CefRange {
    /// Start, in UTF-16 code units.
    pub from: u32,
    /// End (exclusive), in UTF-16 code units.
    pub to: u32,
}

impl CefRange {
    /// `CefRange::InvalidRange()`.
    pub const INVALID: Self = Self {
        from: INVALID_RANGE_BOUND,
        to: INVALID_RANGE_BOUND,
    };

    /// A real range, or [`CefRange::INVALID`] for `None`.
    ///
    /// CEF's C++ side turns a null range pointer into `CefRange()` = `{0, 0}`,
    /// a valid empty range at offset 0, which macOS would honour as "replace
    /// nothing at the start of the field". The sentinel is always passed
    /// explicitly instead.
    pub fn from_option(range: Option<&std::ops::Range<u32>>) -> Self {
        range.map_or(Self::INVALID, |r| Self {
            from: r.start,
            to: r.end,
        })
    }
}

/// One `CefBrowserHost` call, with CEF's argument types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostCall<'a> {
    /// `SendMouseMoveEvent(event, mouse_leave)`.
    MouseMove {
        /// View x, CSS px.
        x: i32,
        /// View y, CSS px.
        y: i32,
        /// `cef_event_flags_t` bits.
        flags: u32,
        /// Pointer left the view.
        leave: bool,
    },
    /// `SendMouseClickEvent(event, type, mouse_up, click_count)`.
    MouseClick {
        /// View x, CSS px.
        x: i32,
        /// View y, CSS px.
        y: i32,
        /// `cef_event_flags_t` bits.
        flags: u32,
        /// `MBT_LEFT` / `MBT_MIDDLE` / `MBT_RIGHT`.
        button: PointerButton,
        /// Release rather than press.
        up: bool,
        /// Consecutive clicks.
        click_count: i32,
    },
    /// `SendMouseWheelEvent(event, delta_x, delta_y)`.
    MouseWheel {
        /// View x, CSS px.
        x: i32,
        /// View y, CSS px.
        y: i32,
        /// `cef_event_flags_t` bits.
        flags: u32,
        /// Horizontal delta, CSS px.
        delta_x: i32,
        /// Vertical delta, CSS px.
        delta_y: i32,
    },
    /// `SendKeyEvent(event)`.
    Key {
        /// `KEYEVENT_RAWKEYDOWN` / `KEYEVENT_KEYUP` / `KEYEVENT_CHAR`.
        kind: KeyEventKind,
        /// `cef_event_flags_t` bits.
        flags: u32,
        /// `windows_key_code`.
        windows_key_code: i32,
        /// `native_key_code`.
        native_key_code: i32,
        /// `character` and `unmodified_character` (one UTF-16 unit).
        character: u16,
    },
    /// `ImeSetComposition(text, underlines = none, replacement, selection)`.
    ImeSetComposition {
        /// Marked text.
        text: &'a str,
        /// Replaced document range.
        replacement: CefRange,
        /// Selection within the composition.
        selection: CefRange,
    },
    /// `ImeCommitText(text, replacement, relative_cursor_pos = 0)`.
    ImeCommit {
        /// Committed text.
        text: &'a str,
        /// Replaced document range.
        replacement: CefRange,
    },
    /// `ImeFinishComposingText(keep_selection)`.
    ImeFinish {
        /// Keep the composition selected.
        keep_selection: bool,
    },
    /// `ImeCancelComposition()`.
    ImeCancel,
}

/// The `cef_event_flags_t` bits for held modifier keys.
pub fn modifier_flags(modifiers: Modifiers) -> u32 {
    [
        (modifiers.shift, flags::SHIFT_DOWN),
        (modifiers.control, flags::CONTROL_DOWN),
        (modifiers.alt, flags::ALT_DOWN),
        (modifiers.meta, flags::COMMAND_DOWN),
    ]
    .into_iter()
    .filter_map(|(held, bit)| held.then_some(bit))
    .fold(0, |acc, bit| acc | bit)
}

fn button_flag(button: PointerButton) -> u32 {
    match button {
        PointerButton::Left => flags::LEFT_MOUSE_BUTTON,
        PointerButton::Middle => flags::MIDDLE_MOUSE_BUTTON,
        PointerButton::Right => flags::RIGHT_MOUSE_BUTTON,
    }
}

/// CEF mouse events carry integer view coordinates; the pixel containing the
/// point is the one under the cursor.
fn view_point(position: Vec2) -> (i32, i32) {
    (position.x.floor() as i32, position.y.floor() as i32)
}

/// Per-page input state the CEF calls need but core events do not carry:
/// which buttons are held (CEF wants them as flags on every mouse event)
/// and sub-pixel wheel remainders (trackpads deliver fractional deltas that
/// would otherwise be truncated away).
#[derive(Debug, Clone, Default)]
pub struct InputTranslator {
    held_buttons: u32,
    wheel_remainder: Vec2,
}

impl InputTranslator {
    /// A translator with no buttons held.
    pub fn new() -> Self {
        Self::default()
    }

    /// The host calls that deliver `event`, in order. Usually one; a
    /// supplementary-plane character becomes two `KEYEVENT_CHAR`s (a UTF-16
    /// surrogate pair), and a wheel event smaller than one pixel becomes none.
    ///
    /// Translation happens eagerly (held buttons and wheel remainders update
    /// now); the result is a fixed two-slot buffer, so the per-event input
    /// path never allocates.
    pub fn translate<'a>(&mut self, event: &'a InputEvent) -> impl Iterator<Item = HostCall<'a>> {
        let calls: [Option<HostCall<'a>>; 2] = match event {
            InputEvent::Pointer(pointer) => [Some(self.pointer(pointer)), None],
            InputEvent::Wheel(wheel) => [self.wheel(wheel), None],
            InputEvent::Key(key) => Self::key(key),
            InputEvent::Ime(ime) => [Some(Self::ime(ime)), None],
        };
        calls.into_iter().flatten()
    }

    fn pointer(&mut self, event: &PointerEvent) -> HostCall<'static> {
        let (x, y) = view_point(event.position);
        let modifiers = modifier_flags(event.modifiers);
        match event.kind {
            PointerEventKind::Move | PointerEventKind::Leave => HostCall::MouseMove {
                x,
                y,
                flags: modifiers | self.held_buttons,
                leave: matches!(event.kind, PointerEventKind::Leave),
            },
            PointerEventKind::Down {
                button,
                click_count,
            } => {
                self.held_buttons |= button_flag(button);
                HostCall::MouseClick {
                    x,
                    y,
                    flags: modifiers | self.held_buttons,
                    button,
                    up: false,
                    click_count: i32::from(click_count),
                }
            }
            PointerEventKind::Up {
                button,
                click_count,
            } => {
                self.held_buttons &= !button_flag(button);
                HostCall::MouseClick {
                    x,
                    y,
                    flags: modifiers | self.held_buttons,
                    button,
                    up: true,
                    click_count: i32::from(click_count),
                }
            }
        }
    }

    fn wheel(&mut self, event: &WheelEvent) -> Option<HostCall<'static>> {
        let total = self.wheel_remainder + event.delta;
        let whole = total.trunc();
        self.wheel_remainder = total - whole;
        if whole == Vec2::ZERO {
            return None;
        }
        let (x, y) = view_point(event.position);
        Some(HostCall::MouseWheel {
            x,
            y,
            flags: modifier_flags(event.modifiers) | self.held_buttons,
            delta_x: whole.x as i32,
            delta_y: whole.y as i32,
        })
    }

    fn key(event: &KeyEvent) -> [Option<HostCall<'static>>; 2] {
        let flags = modifier_flags(event.modifiers);
        let mut units = [0_u16; 2];
        let encoded: &[u16] = match event.character {
            Some(ch) => ch.encode_utf16(&mut units),
            None => &[0],
        };
        let (first, second) = match *encoded {
            [first, second] => (first, Some(second)),
            [first, ..] => (first, None),
            [] => (0, None),
        };
        let call = |character: u16| HostCall::Key {
            kind: event.kind,
            flags,
            windows_key_code: event.windows_key_code,
            native_key_code: event.native_key_code,
            character,
        };
        match event.kind {
            // A character event is one per UTF-16 unit; Chromium reassembles
            // surrogate pairs, as it does for Windows WM_CHAR pairs.
            KeyEventKind::Char => [Some(call(first)), second.map(call)],
            KeyEventKind::RawDown | KeyEventKind::Up => [Some(call(first)), None],
        }
    }

    fn ime(event: &ImeEvent) -> HostCall<'_> {
        match event {
            ImeEvent::SetComposition {
                text,
                selection,
                replacement,
            } => HostCall::ImeSetComposition {
                text,
                replacement: CefRange::from_option(replacement.as_ref()),
                selection: CefRange::from_option(Some(selection)),
            },
            ImeEvent::Commit { text, replacement } => HostCall::ImeCommit {
                text,
                replacement: CefRange::from_option(replacement.as_ref()),
            },
            ImeEvent::FinishComposing { keep_selection } => HostCall::ImeFinish {
                keep_selection: *keep_selection,
            },
            ImeEvent::Cancel => HostCall::ImeCancel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calls<'a>(translator: &mut InputTranslator, event: &'a InputEvent) -> Vec<HostCall<'a>> {
        translator.translate(event).collect()
    }

    fn pointer(kind: PointerEventKind, x: f32, y: f32) -> InputEvent {
        InputEvent::Pointer(PointerEvent {
            kind,
            position: Vec2::new(x, y),
            modifiers: Modifiers::default(),
        })
    }

    fn wheel(dx: f32, dy: f32) -> InputEvent {
        InputEvent::Wheel(WheelEvent {
            position: Vec2::new(5.0, 5.0),
            delta: Vec2::new(dx, dy),
            modifiers: Modifiers::default(),
        })
    }

    fn key_char(ch: char) -> InputEvent {
        InputEvent::Key(KeyEvent {
            kind: KeyEventKind::Char,
            windows_key_code: 0,
            native_key_code: 0,
            character: Some(ch),
            modifiers: Modifiers::default(),
        })
    }

    #[test]
    fn modifier_flags_match_cef_bits() {
        let all = Modifiers {
            shift: true,
            control: true,
            alt: true,
            meta: true,
        };
        assert_eq!(modifier_flags(all), 2 | 4 | 8 | 128);
    }

    #[test]
    fn move_while_left_button_held_carries_left_button_flag() {
        let mut translator = InputTranslator::new();
        let down = pointer(
            PointerEventKind::Down {
                button: PointerButton::Left,
                click_count: 1,
            },
            1.0,
            1.0,
        );
        calls(&mut translator, &down);
        let moved = pointer(PointerEventKind::Move, 2.0, 2.0);
        assert_eq!(
            calls(&mut translator, &moved),
            vec![HostCall::MouseMove {
                x: 2,
                y: 2,
                flags: flags::LEFT_MOUSE_BUTTON,
                leave: false
            }]
        );
    }

    #[test]
    fn mouse_up_clears_the_released_button_flag() {
        let mut translator = InputTranslator::new();
        let button = PointerButton::Right;
        calls(
            &mut translator,
            &pointer(
                PointerEventKind::Down {
                    button,
                    click_count: 1,
                },
                0.0,
                0.0,
            ),
        );
        let up = pointer(
            PointerEventKind::Up {
                button,
                click_count: 1,
            },
            0.0,
            0.0,
        );
        assert!(matches!(
            calls(&mut translator, &up).as_slice(),
            [HostCall::MouseClick {
                flags: 0,
                up: true,
                ..
            }]
        ));
    }

    #[test]
    fn fractional_position_floors_to_containing_pixel() {
        let mut translator = InputTranslator::new();
        let event = pointer(PointerEventKind::Move, -0.5, 10.9);
        let calls = calls(&mut translator, &event);
        assert!(matches!(
            calls.as_slice(),
            [HostCall::MouseMove { x: -1, y: 10, .. }]
        ));
    }

    #[test]
    fn sub_pixel_wheel_deltas_accumulate_until_a_whole_pixel() {
        let mut translator = InputTranslator::new();
        let event = wheel(0.0, 0.6);
        let first = calls(&mut translator, &event).len();
        let second = calls(&mut translator, &event);
        assert_eq!(
            (first, second.as_slice()),
            (
                0,
                [HostCall::MouseWheel {
                    x: 5,
                    y: 5,
                    flags: 0,
                    delta_x: 0,
                    delta_y: 1
                }]
                .as_slice()
            )
        );
    }

    #[test]
    fn astral_character_is_a_surrogate_pair_of_char_events() {
        let mut translator = InputTranslator::new();
        let characters: Vec<u16> = translator
            .translate(&key_char('😀'))
            .filter_map(|call| match call {
                HostCall::Key { character, .. } => Some(character),
                _ => None,
            })
            .collect();
        assert_eq!(characters, vec![0xD83D, 0xDE00]);
    }

    #[test]
    fn raw_key_down_without_character_sends_zero_character() {
        let mut translator = InputTranslator::new();
        let event = InputEvent::Key(KeyEvent {
            kind: KeyEventKind::RawDown,
            windows_key_code: 0x25,
            native_key_code: 0x7B,
            character: None,
            modifiers: Modifiers::default(),
        });
        assert_eq!(
            calls(&mut translator, &event),
            vec![HostCall::Key {
                kind: KeyEventKind::RawDown,
                flags: 0,
                windows_key_code: 0x25,
                native_key_code: 0x7B,
                character: 0
            }]
        );
    }

    #[test]
    fn composition_without_replacement_passes_invalid_range() {
        let mut translator = InputTranslator::new();
        let event = InputEvent::Ime(ImeEvent::SetComposition {
            text: "かな".to_owned(),
            selection: 2..2,
            replacement: None,
        });
        assert_eq!(
            calls(&mut translator, &event),
            vec![HostCall::ImeSetComposition {
                text: "かな",
                replacement: CefRange::INVALID,
                selection: CefRange { from: 2, to: 2 },
            }]
        );
    }
}
