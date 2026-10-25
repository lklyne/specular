//! A binding table chord as a native menu shortcut.

use muda::accelerator::{Accelerator, CMD_OR_CTRL, Code, Modifiers};
use specular_interact::{Chord, Key};

const LETTERS: [Code; 26] = [
    Code::KeyA,
    Code::KeyB,
    Code::KeyC,
    Code::KeyD,
    Code::KeyE,
    Code::KeyF,
    Code::KeyG,
    Code::KeyH,
    Code::KeyI,
    Code::KeyJ,
    Code::KeyK,
    Code::KeyL,
    Code::KeyM,
    Code::KeyN,
    Code::KeyO,
    Code::KeyP,
    Code::KeyQ,
    Code::KeyR,
    Code::KeyS,
    Code::KeyT,
    Code::KeyU,
    Code::KeyV,
    Code::KeyW,
    Code::KeyX,
    Code::KeyY,
    Code::KeyZ,
];

const DIGITS: [Code; 10] = [
    Code::Digit0,
    Code::Digit1,
    Code::Digit2,
    Code::Digit3,
    Code::Digit4,
    Code::Digit5,
    Code::Digit6,
    Code::Digit7,
    Code::Digit8,
    Code::Digit9,
];

/// The shortcut to show beside a menu item bound to `chord`, or `None` for
/// a key a menu cannot show.
pub(super) fn accelerator(chord: Chord) -> Option<Accelerator> {
    let mut modifiers = Modifiers::empty();
    if chord.cmd {
        modifiers |= CMD_OR_CTRL;
    }
    if chord.shift {
        modifiers |= Modifiers::SHIFT;
    }
    if chord.alt {
        modifiers |= Modifiers::ALT;
    }
    Some(Accelerator::new(modifiers, code(chord.key)?))
}

fn code(key: Key) -> Option<Code> {
    Some(match key {
        Key::Escape => Code::Escape,
        Key::Enter => Code::Enter,
        Key::Tab => Code::Tab,
        Key::Backspace => Code::Backspace,
        Key::Delete => Code::Delete,
        Key::Home => Code::Home,
        Key::End => Code::End,
        Key::PageUp => Code::PageUp,
        Key::PageDown => Code::PageDown,
        Key::Space => Code::Space,
        Key::ArrowLeft => Code::ArrowLeft,
        Key::ArrowRight => Code::ArrowRight,
        Key::ArrowUp => Code::ArrowUp,
        Key::ArrowDown => Code::ArrowDown,
        Key::Char(character) => return character_code(character),
        Key::Other => return None,
    })
}

/// The key that types `character` on a US layout, unshifted.
fn character_code(character: char) -> Option<Code> {
    let index = |first: char| (u32::from(character)).checked_sub(u32::from(first));
    Some(match character {
        'a'..='z' => *LETTERS.get(index('a')? as usize)?,
        '0'..='9' => *DIGITS.get(index('0')? as usize)?,
        '-' => Code::Minus,
        '=' => Code::Equal,
        '[' => Code::BracketLeft,
        ']' => Code::BracketRight,
        '\\' => Code::Backslash,
        ';' => Code::Semicolon,
        '\'' => Code::Quote,
        ',' => Code::Comma,
        '.' => Code::Period,
        '/' => Code::Slash,
        '`' => Code::Backquote,
        _ => return None,
    })
}
