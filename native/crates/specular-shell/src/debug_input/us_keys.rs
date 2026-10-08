//! Keys as a US keyboard sends them: the `kVK_*` code of each key, and the
//! characters `AppKit` puts in its event.

use specular_core::Modifiers;
use specular_interact::Key;

use super::steps::Do;

/// `NSEventModifierFlag` bits.
pub(super) const SHIFT: usize = 1 << 17;
pub(super) const CONTROL: usize = 1 << 18;
pub(super) const OPTION: usize = 1 << 19;
pub(super) const COMMAND: usize = 1 << 20;

/// The `kVK_*` code and whether shift is held for a character of the US
/// layout.
pub(super) fn key_of(character: char) -> Option<(u16, bool)> {
    const PLAIN: &str = "asdfhgzxcv\u{0}bqweryt123465=97-80]ou[ip\rlj'k;\\,/nm.\t `";
    const SHIFTED: &str = "ASDFHGZXCV\u{0}BQWERYT!@#$^%+(&_*)}OU{IP\rLJ\"K:|<?NM>\t ~";
    let find = |keys: &str| {
        (keys.chars().position(|key| key == character)).and_then(|at| u16::try_from(at).ok())
    };
    find(PLAIN)
        .map(|code| (code, false))
        .or_else(|| find(SHIFTED).map(|code| (code, true)))
}

/// The code of a key that types nothing, and the character `AppKit` sends
/// with it.
const fn named_key(key: Key) -> Option<(u16, char)> {
    Some(match key {
        Key::Escape => (53, '\u{1b}'),
        Key::Enter => (36, '\r'),
        Key::Tab => (48, '\t'),
        Key::Backspace => (51, '\u{7f}'),
        Key::Delete => (117, '\u{f728}'),
        Key::Home => (115, '\u{f729}'),
        Key::End => (119, '\u{f72b}'),
        Key::PageUp => (116, '\u{f72c}'),
        Key::PageDown => (121, '\u{f72d}'),
        Key::Space => (49, ' '),
        Key::ArrowLeft => (123, '\u{f702}'),
        Key::ArrowRight => (124, '\u{f703}'),
        Key::ArrowUp => (126, '\u{f700}'),
        Key::ArrowDown => (125, '\u{f701}'),
        Key::Char(_) | Key::Other => return None,
    })
}

pub(super) const fn flags_of(held: Modifiers) -> usize {
    (if held.shift { SHIFT } else { 0 })
        | (if held.control { CONTROL } else { 0 })
        | (if held.alt { OPTION } else { 0 })
        | (if held.meta { COMMAND } else { 0 })
}

/// `key` with `held` as the key event a US keyboard sends for it.
pub(super) fn key_step(held: Modifiers, key: Key) -> Result<Do, String> {
    let flags = flags_of(held);
    if let Some((code, character)) = named_key(key) {
        return Ok(Do::Key {
            code,
            flags,
            characters: character.to_string(),
            plain: character.to_string(),
        });
    }
    let Key::Char(character) = key else {
        return Err("a key the window has no code for".to_owned());
    };
    let (code, _) =
        key_of(character).ok_or_else(|| format!("no key of a US keyboard types `{character}`"))?;
    let shifted = if held.shift {
        character.to_uppercase().to_string()
    } else {
        character.to_string()
    };
    // Command and Control keep the plain character, as `AppKit` has it.
    let characters = if held.meta || held.control {
        character.to_string()
    } else {
        shifted.clone()
    };
    Ok(Do::Key {
        code,
        flags,
        characters,
        plain: shifted,
    })
}
