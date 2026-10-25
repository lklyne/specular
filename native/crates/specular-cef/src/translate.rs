//! Translation of [`InputEvent`]s into the exact CEF `BrowserHost` calls.
//!
//! The CEF backend's `send_input` is a field-for-field copy of a
//! [`HostCall`]; every decision (flag bits, rounding, held buttons, wheel
//! remainders, UTF-16 splitting, IME range sentinels) lives here, where it is
//! unit-tested without CEF.

use glam::Vec2;
use specular_core::{
    EditingKey, ImeEvent, InputEvent, KeyEvent, KeyEventKind, Modifiers, PageEdit, PointerButton,
    PointerEvent, PointerEventKind, TouchPhase, WheelEvent,
};

use crate::key_message::PlainKey;

/// CEF `cef_event_flags_t` bits (`include/internal/cef_types.h`).
pub(crate) mod flags {
    /// `EVENTFLAG_SHIFT_DOWN`.
    pub(crate) const SHIFT_DOWN: u32 = 1 << 1;
    /// `EVENTFLAG_CONTROL_DOWN`.
    pub(crate) const CONTROL_DOWN: u32 = 1 << 2;
    /// `EVENTFLAG_ALT_DOWN`.
    pub(crate) const ALT_DOWN: u32 = 1 << 3;
    /// `EVENTFLAG_LEFT_MOUSE_BUTTON`.
    pub(crate) const LEFT_MOUSE_BUTTON: u32 = 1 << 4;
    /// `EVENTFLAG_MIDDLE_MOUSE_BUTTON`.
    pub(crate) const MIDDLE_MOUSE_BUTTON: u32 = 1 << 5;
    /// `EVENTFLAG_RIGHT_MOUSE_BUTTON`.
    pub(crate) const RIGHT_MOUSE_BUTTON: u32 = 1 << 6;
    /// `EVENTFLAG_COMMAND_DOWN` (macOS Command).
    pub(crate) const COMMAND_DOWN: u32 = 1 << 7;
}

/// CEF's `CefRange::InvalidRange()` component, meaning "no range".
pub(crate) const INVALID_RANGE_BOUND: u32 = u32::MAX;

/// A UTF-16 range as CEF's `cef_range_t` (`from`, `to`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CefRange {
    /// Start, in UTF-16 code units.
    pub(crate) from: u32,
    /// End (exclusive), in UTF-16 code units.
    pub(crate) to: u32,
}

impl CefRange {
    /// `CefRange::InvalidRange()`.
    pub(crate) const INVALID: Self = Self {
        from: INVALID_RANGE_BOUND,
        to: INVALID_RANGE_BOUND,
    };

    /// A real range, or [`CefRange::INVALID`] for `None`.
    ///
    /// CEF's C++ side turns a null range pointer into `CefRange()` = `{0, 0}`,
    /// a valid empty range at offset 0, which macOS would honour as "replace
    /// nothing at the start of the field". The sentinel is always passed
    /// explicitly instead.
    pub(crate) fn from_option(range: Option<&std::ops::Range<u32>>) -> Self {
        range.map_or(Self::INVALID, |r| Self {
            from: r.start,
            to: r.end,
        })
    }
}

/// One `CefBrowserHost` call, with CEF's argument types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HostCall<'a> {
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
    /// `SendTouchEvent(event)`: one finger.
    Touch {
        /// What the finger did.
        phase: TouchPhase,
        /// View x, CSS px.
        x: i32,
        /// View y, CSS px.
        y: i32,
        /// `cef_event_flags_t` bits.
        flags: u32,
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
    /// `Input.dispatchKeyEvent` over the devtools channel: a raw key-down
    /// with its editing commands, which `SendKeyEvent` cannot carry. The
    /// plain call is made instead if the page refuses the message.
    EditingKey {
        /// The key as `SendKeyEvent` would take it.
        key: PlainKey,
        /// The commands and the key's DOM names.
        editing: &'a EditingKey,
    },
    /// An Edit menu command on the focused frame (`CefFrame::Undo` and its
    /// like).
    Edit(PageEdit),
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
pub(crate) fn modifier_flags(modifiers: Modifiers) -> u32 {
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
pub(crate) struct InputTranslator {
    held_buttons: u32,
    wheel_remainder: Vec2,
}

impl InputTranslator {
    /// A translator with no buttons held.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The host calls that deliver `event`, in order. Usually one; a
    /// supplementary-plane character becomes two `KEYEVENT_CHAR`s (a UTF-16
    /// surrogate pair), and a wheel event smaller than one pixel becomes none.
    ///
    /// Translation happens eagerly (held buttons and wheel remainders update
    /// now); the result is a fixed two-slot buffer, so the per-event input
    /// path never allocates.
    pub(crate) fn translate<'a>(
        &mut self,
        event: &'a InputEvent,
    ) -> impl Iterator<Item = HostCall<'a>> {
        let calls: [Option<HostCall<'a>>; 2] = match event {
            InputEvent::Pointer(pointer) => [Some(self.pointer(pointer)), None],
            InputEvent::Touch(touch) => {
                let (x, y) = view_point(touch.position);
                let flags = modifier_flags(touch.modifiers);
                let phase = touch.phase;
                [Some(HostCall::Touch { phase, x, y, flags }), None]
            }
            InputEvent::Wheel(wheel) => [self.wheel(wheel), None],
            InputEvent::Key(key) => Self::key(key),
            InputEvent::Ime(ime) => [Some(Self::ime(ime)), None],
            InputEvent::Edit(edit) => [Some(HostCall::Edit(*edit)), None],
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

    fn key(event: &KeyEvent) -> [Option<HostCall<'_>>; 2] {
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
        if let (KeyEventKind::RawDown, Some(editing)) = (event.kind, &event.editing) {
            let key = PlainKey {
                flags,
                windows_key_code: event.windows_key_code,
                native_key_code: event.native_key_code,
                character: first,
            };
            return [Some(HostCall::EditingKey { key, editing }), None];
        }
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

    fn key_char(ch: char) -> InputEvent {
        InputEvent::Key(KeyEvent {
            kind: KeyEventKind::Char,
            windows_key_code: 0,
            native_key_code: 0,
            character: Some(ch),
            modifiers: Modifiers::default(),
            editing: None,
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
}
